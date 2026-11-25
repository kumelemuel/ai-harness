use super::{CreateCommunityCommand, CreateCommunityError, CreateCommunityResponse};
use crate::errors::{Conflict, ContentRejected, DependencyUnavailable, Validation};
use crate::ports::inbound::CreateCommunityUseCase;
use crate::ports::outbound::dtos::{ModerationError, ModerationVerdict, ReviewContentRequest};
use crate::ports::outbound::{CommunityEventPublisher, CommunityRepository, ModerationGateway};
use communities_domain::Community;
use communities_domain::value_objects::{CommunityId, CommunityName, CommunitySlug, OwnerId};
use kernel::{BoxFuture, Failure, IdGenerator};
use std::sync::Arc;

/// Orquesta (HEX-7): lee el input, carga, invoca al dominio, persiste y mapea.
///
/// Orden: value objects, conflicto de slug, moderación, guardado y publicación.
/// La moderación es un tercero (lenta, falible): va después de lo local y barato.
/// Los eventos se publican después de guardar, y solo si se guardó; con un
/// `UnitOfWork`, las dos cosas van en la misma transacción.
pub struct CreateCommunityHandler {
    communities: Arc<dyn CommunityRepository>,
    moderation: Arc<dyn ModerationGateway>,
    ids: Arc<dyn IdGenerator>,
    events: Arc<dyn CommunityEventPublisher>,
}

impl CreateCommunityHandler {
    pub fn new(
        communities: Arc<dyn CommunityRepository>,
        moderation: Arc<dyn ModerationGateway>,
        ids: Arc<dyn IdGenerator>,
        events: Arc<dyn CommunityEventPublisher>,
    ) -> Self {
        Self {
            communities,
            moderation,
            ids,
            events,
        }
    }
}

type Outcome = Result<CreateCommunityResponse, Failure<CreateCommunityError>>;

impl CreateCommunityUseCase for CreateCommunityHandler {
    fn execute(&self, command: CreateCommunityCommand) -> BoxFuture<'_, Outcome> {
        Box::pin(async move {
            // Errores de dominio → hoja plana del contrato, sin envolver (ERR-2).
            let slug = CommunitySlug::new(command.slug)
                .map_err(|_| Failure::expected(Validation { field: "slug" }))?;
            let name = CommunityName::new(command.name)
                .map_err(|_| Failure::expected(Validation { field: "name" }))?;

            // `find_*` devuelve Option; el caso de uso decide que existir es un
            // conflicto (ERR-1). `?` propaga el `Fault` como árbol B.
            if self.communities.find_by_slug(&slug).await?.is_some() {
                return Err(Failure::expected(Conflict {
                    resource: "community",
                }));
            }

            // Rechazo: éxito del puerto que este caso de uso trata como error.
            // No disponible: el error declarado, a su hoja (ERR-2, ERR-3).
            let verdict = self
                .moderation
                .review_content(ReviewContentRequest {
                    content: name.as_str().to_string(),
                })
                .await
                .map_err(|failure| {
                    failure.map_expected(|error| match error {
                        ModerationError::Unavailable => {
                            CreateCommunityError::from(DependencyUnavailable {
                                dependency: "moderation",
                            })
                        }
                    })
                })?;
            if let ModerationVerdict::Rejected { reason } = verdict {
                return Err(Failure::expected(ContentRejected {
                    field: "name",
                    reason,
                }));
            }

            // El agregado registra `Created`; se guarda sin eventos pendientes
            // (no se persisten) y se publican después del guardado.
            let (community, events) = Community::create(
                CommunityId::new(self.ids.next_id()),
                OwnerId::new(command.owner_id),
                slug,
                name,
                command.is_public,
            )
            .take_events();

            self.communities.save(&community).await?;
            self.events.publish(&events).await?;

            Ok(CreateCommunityResponse::from(&community))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{ContentRejected, DependencyUnavailable};
    use crate::testing::{
        CommunityOperation, FakeCommunityAdapter, FakeModerationAdapter, FixedIdGeneratorAdapter,
        SpyCommunityEventAdapter, block_on,
    };
    use communities_domain::CommunityEvent;

    fn command(slug: &str, name: &str) -> CreateCommunityCommand {
        CreateCommunityCommand {
            owner_id: "alice".into(),
            slug: slug.into(),
            name: name.into(),
            is_public: true,
        }
    }

    fn handler(repository: &Arc<FakeCommunityAdapter>) -> CreateCommunityHandler {
        moderated(repository, &Arc::new(FakeModerationAdapter::approving()))
    }

    fn moderated(
        repository: &Arc<FakeCommunityAdapter>,
        moderation: &Arc<FakeModerationAdapter>,
    ) -> CreateCommunityHandler {
        publishing(
            repository,
            moderation,
            &Arc::new(SpyCommunityEventAdapter::default()),
        )
    }

    fn publishing(
        repository: &Arc<FakeCommunityAdapter>,
        moderation: &Arc<FakeModerationAdapter>,
        events: &Arc<SpyCommunityEventAdapter>,
    ) -> CreateCommunityHandler {
        CreateCommunityHandler::new(
            repository.clone(),
            moderation.clone(),
            Arc::new(FixedIdGeneratorAdapter("c-1")),
            events.clone(),
        )
    }

    #[test]
    fn creates_and_saves_the_community() {
        let repository = Arc::new(FakeCommunityAdapter::empty());

        let response = block_on(handler(&repository).execute(command("rust-chile", "Rust Chile")));

        assert_eq!(
            response.unwrap(),
            CreateCommunityResponse {
                id: "c-1".into(),
                slug: "rust-chile".into(),
                name: "Rust Chile".into(),
                is_public: true,
            }
        );
        assert_eq!(repository.saved().len(), 1);
    }

    #[test]
    fn publishes_created_and_saves_without_pending_events() {
        let repository = Arc::new(FakeCommunityAdapter::empty());
        let events = Arc::new(SpyCommunityEventAdapter::default());
        let handler = publishing(
            &repository,
            &Arc::new(FakeModerationAdapter::approving()),
            &events,
        );

        block_on(handler.execute(command("rust-chile", "Rust Chile"))).unwrap();

        assert!(matches!(
            events.published().as_slice(),
            [CommunityEvent::Created { community_id, owner }]
                if community_id.as_str() == "c-1" && owner.as_str() == "alice"
        ));
        assert!(repository.saved()[0].events().is_empty(), "no se persisten");
    }

    #[test]
    fn nothing_is_published_when_the_community_is_not_saved() {
        let events = Arc::new(SpyCommunityEventAdapter::default());
        let rejected = publishing(
            &Arc::new(FakeCommunityAdapter::empty()),
            &Arc::new(FakeModerationAdapter::rejecting()),
            &events,
        );
        let failing = publishing(
            &Arc::new(FakeCommunityAdapter::empty().failing_on(CommunityOperation::Save)),
            &Arc::new(FakeModerationAdapter::approving()),
            &events,
        );

        let _ = block_on(rejected.execute(command("rust-chile", "Rust Chile")));
        let _ = block_on(failing.execute(command("rust-chile", "Rust Chile")));

        assert!(events.published().is_empty());
    }

    #[test]
    fn invalid_slug_is_a_validation_error() {
        let repository = Arc::new(FakeCommunityAdapter::empty());

        let result = block_on(handler(&repository).execute(command("no spaces", "Rust Chile")));

        assert!(matches!(
            result,
            Err(Failure::Expected(CreateCommunityError::Validation(
                Validation { field: "slug" }
            )))
        ));
        assert!(repository.saved().is_empty());
    }

    #[test]
    fn invalid_name_is_a_validation_error() {
        let repository = Arc::new(FakeCommunityAdapter::empty());

        let result = block_on(handler(&repository).execute(command("rust-chile", "ab")));

        assert!(matches!(
            result,
            Err(Failure::Expected(CreateCommunityError::Validation(
                Validation { field: "name" }
            )))
        ));
    }

    #[test]
    fn existing_slug_is_a_conflict() {
        let repository = Arc::new(FakeCommunityAdapter::empty());
        block_on(handler(&repository).execute(command("rust-chile", "Rust Chile"))).unwrap();

        let result = block_on(handler(&repository).execute(command("rust-chile", "Otra")));

        assert!(matches!(
            result,
            Err(Failure::Expected(CreateCommunityError::Conflict(_)))
        ));
    }

    #[test]
    fn fails_with_unexpected_when_find_by_slug_fails() {
        let repository = Arc::new(FakeCommunityAdapter::failing());

        let result = block_on(handler(&repository).execute(command("rust-chile", "Rust Chile")));

        assert!(matches!(result, Err(Failure::Unexpected(_))));
    }

    /// Una prueba de falla por cada llamada a un puerto que puede fallar
    /// (TDD-2): `failing()` corta en `find_by_slug`, y `save` necesita que
    /// la búsqueda cumpla el contrato.
    #[test]
    fn fails_with_unexpected_when_save_fails() {
        let repository =
            Arc::new(FakeCommunityAdapter::empty().failing_on(CommunityOperation::Save));
        let events = Arc::new(SpyCommunityEventAdapter::default());
        let handler = publishing(
            &repository,
            &Arc::new(FakeModerationAdapter::approving()),
            &events,
        );

        let result = block_on(handler.execute(command("rust-chile", "Rust Chile")));

        assert!(matches!(result, Err(Failure::Unexpected(_))));
        assert!(events.published().is_empty(), "no publica lo que no guardó");
    }

    #[test]
    fn fails_with_unexpected_when_publish_fails() {
        let events = Arc::new(SpyCommunityEventAdapter::failing());
        let handler = publishing(
            &Arc::new(FakeCommunityAdapter::empty()),
            &Arc::new(FakeModerationAdapter::approving()),
            &events,
        );

        let result = block_on(handler.execute(command("rust-chile", "Rust Chile")));

        assert!(matches!(result, Err(Failure::Unexpected(_))));
    }

    #[test]
    fn approved_content_is_moderated_and_saved() {
        let repository = Arc::new(FakeCommunityAdapter::empty());
        let moderation = Arc::new(FakeModerationAdapter::approving());

        let result = block_on(
            moderated(&repository, &moderation).execute(command("rust-chile", "Rust Chile")),
        );

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(moderation.reviewed(), vec!["Rust Chile".to_string()]);
        assert_eq!(repository.saved().len(), 1);
    }

    #[test]
    fn rejected_content_is_content_rejected_and_not_saved() {
        let repository = Arc::new(FakeCommunityAdapter::empty());
        let moderation = Arc::new(FakeModerationAdapter::rejecting());

        let result = block_on(
            moderated(&repository, &moderation).execute(command("rust-chile", "Rust Chile")),
        );

        assert!(
            matches!(
                &result,
                Err(Failure::Expected(CreateCommunityError::ContentRejected(
                    ContentRejected { field: "name", reason }
                ))) if !reason.is_empty()
            ),
            "{result:?}"
        );
        assert!(repository.saved().is_empty());
    }

    #[test]
    fn unavailable_moderation_is_dependency_unavailable_and_not_saved() {
        let repository = Arc::new(FakeCommunityAdapter::empty());
        let moderation = Arc::new(FakeModerationAdapter::unavailable());

        let result = block_on(
            moderated(&repository, &moderation).execute(command("rust-chile", "Rust Chile")),
        );

        assert!(
            matches!(
                result,
                Err(Failure::Expected(
                    CreateCommunityError::DependencyUnavailable(DependencyUnavailable {
                        dependency: "moderation"
                    })
                ))
            ),
            "{result:?}"
        );
        assert!(repository.saved().is_empty());
    }

    /// Lo local y barato va antes: con un input inválido o un slug tomado no se
    /// consulta al tercero.
    #[test]
    fn moderation_runs_after_local_checks() {
        let repository = Arc::new(FakeCommunityAdapter::empty());
        block_on(handler(&repository).execute(command("rust-chile", "Rust Chile"))).unwrap();
        let moderation = Arc::new(FakeModerationAdapter::approving());
        let handler = moderated(&repository, &moderation);

        let invalid = block_on(handler.execute(command("no spaces", "Rust Chile")));
        let taken = block_on(handler.execute(command("rust-chile", "Otra")));

        assert!(matches!(
            invalid,
            Err(Failure::Expected(CreateCommunityError::Validation(_)))
        ));
        assert!(matches!(
            taken,
            Err(Failure::Expected(CreateCommunityError::Conflict(_)))
        ));
        assert!(moderation.reviewed().is_empty());
    }
}
