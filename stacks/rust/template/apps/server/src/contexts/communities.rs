use communities_api::CommunitiesApi;
use communities_application::commands::create_community::CreateCommunityHandler;
use communities_infrastructure::{
    InMemoryCommunityAdapter, NoOpModerationAdapter, UuidIdGeneratorAdapter,
};
use kernel::FaultReporter;
use std::sync::Arc;

pub fn api(faults: Arc<dyn FaultReporter>) -> CommunitiesApi {
    let repository = Arc::new(InMemoryCommunityAdapter::default());
    // Sin servicio de moderación real todavía: aprueba todo.
    let moderation = Arc::new(NoOpModerationAdapter);
    let ids = Arc::new(UuidIdGeneratorAdapter);

    CommunitiesApi {
        create_community: Arc::new(CreateCommunityHandler::new(repository, moderation, ids)),
        faults,
    }
}
