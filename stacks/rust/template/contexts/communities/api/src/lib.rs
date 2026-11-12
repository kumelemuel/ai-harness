//! Adaptadores driving HTTP del contexto `communities` (ERR-4: el transporte
//! se traduce solo aquí).

mod codes;
mod create_community;

pub use create_community::{CreateCommunityHttpRequest, CreateCommunityHttpResponse};

use axum::Router;
use axum::routing::post;
use communities_application::ports::inbound::CreateCommunityUseCase;
use kernel::FaultReporter;
use std::sync::Arc;

/// Lo que este adaptador necesita: contratos (`UseCase`), nunca implementaciones (HEX-4).
#[derive(Clone)]
pub struct CommunitiesApi {
    pub create_community: Arc<dyn CreateCommunityUseCase>,
    pub faults: Arc<dyn FaultReporter>,
}

impl CommunitiesApi {
    /// Rutas que requieren identidad (`AuthenticatedAccount`). El composition root
    /// les aplica el middleware de autenticación.
    pub fn protected(self) -> Router {
        Router::new()
            .route("/communities", post(create_community::handle))
            .with_state(self)
    }
}
