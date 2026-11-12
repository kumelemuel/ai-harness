use super::{CreateCommunityCommand, CreateCommunityError, CreateCommunityResponse};
use crate::errors::{Conflict, Validation};
use crate::ports::inbound::CreateCommunityUseCase;
use crate::ports::outbound::CommunityRepository;
use communities_domain::Community;
use communities_domain::value_objects::{CommunityId, CommunityName, CommunitySlug, OwnerId};
use kernel::{BoxFuture, Failure, IdGenerator};
use std::sync::Arc;

/// Orquesta (HEX-7): lee el input, carga, invoca al dominio, persiste y mapea.
pub struct CreateCommunityHandler {
    communities: Arc<dyn CommunityRepository>,
    ids: Arc<dyn IdGenerator>,
}

impl CreateCommunityHandler {
    pub fn new(communities: Arc<dyn CommunityRepository>, ids: Arc<dyn IdGenerator>) -> Self {
        Self { communities, ids }
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

            let community = Community::create(
                CommunityId::new(self.ids.next_id()),
                OwnerId::new(command.owner_id),
                slug,
                name,
                command.is_public,
            );

            self.communities.save(&community).await?;

            Ok(CreateCommunityResponse::from(&community))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeCommunityRepository, FixedIdGenerator, block_on};

    fn command(slug: &str, name: &str) -> CreateCommunityCommand {
        CreateCommunityCommand {
            owner_id: "alice".into(),
            slug: slug.into(),
            name: name.into(),
            is_public: true,
        }
    }

    fn handler(repository: &Arc<FakeCommunityRepository>) -> CreateCommunityHandler {
        CreateCommunityHandler::new(repository.clone(), Arc::new(FixedIdGenerator("c-1")))
    }

    #[test]
    fn creates_and_saves_the_community() {
        let repository = Arc::new(FakeCommunityRepository::empty());

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
    fn invalid_slug_is_a_validation_error() {
        let repository = Arc::new(FakeCommunityRepository::empty());

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
        let repository = Arc::new(FakeCommunityRepository::empty());

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
        let repository = Arc::new(FakeCommunityRepository::empty());
        block_on(handler(&repository).execute(command("rust-chile", "Rust Chile"))).unwrap();

        let result = block_on(handler(&repository).execute(command("rust-chile", "Otra")));

        assert!(matches!(
            result,
            Err(Failure::Expected(CreateCommunityError::Conflict(_)))
        ));
    }

    #[test]
    fn repository_failure_is_unexpected_not_a_business_error() {
        let repository = Arc::new(FakeCommunityRepository::failing());

        let result = block_on(handler(&repository).execute(command("rust-chile", "Rust Chile")));

        assert!(matches!(result, Err(Failure::Unexpected(_))));
    }
}
