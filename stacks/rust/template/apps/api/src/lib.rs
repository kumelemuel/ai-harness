//! Adaptadores driving HTTP. Los códigos de transporte existen solo aquí (ERR-4).

pub mod communities;
pub mod problem;

use axum::Router;
use axum::routing::post;
use communities_application::ports::inbound::CreateCommunityUseCase;
use kernel::FaultReporter;
use std::sync::Arc;

/// Lo que `api` necesita: contratos (`UseCase`), nunca implementaciones (HEX-4).
#[derive(Clone)]
pub struct AppState {
    pub create_community: Arc<dyn CreateCommunityUseCase>,
    pub faults: Arc<dyn FaultReporter>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/communities", post(communities::create::create_community))
        .with_state(state)
}
