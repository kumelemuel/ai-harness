//! Adaptadores driving HTTP del contexto `iam` (ERR-4: el transporte se
//! traduce solo aquí).

mod authenticate;
mod codes;

use axum::Router;
use axum::middleware::from_fn_with_state;
use iam_application::ports::inbound::VerifyAccessTokenUseCase;
use kernel::FaultReporter;
use std::sync::Arc;

/// Lo que este adaptador necesita: contratos (`UseCase`), nunca implementaciones (HEX-4).
#[derive(Clone)]
pub struct IamApi {
    pub verify_access_token: Arc<dyn VerifyAccessTokenUseCase>,
    pub faults: Arc<dyn FaultReporter>,
}

impl IamApi {
    /// Protege las rutas de cualquier contexto: exige `Authorization: Bearer
    /// <token>`, lo verifica y entrega la cuenta como
    /// `edge_http::AuthenticatedAccount`. Sin token válido, responde 401 sin
    /// llegar a la ruta.
    ///
    /// El composition root la aplica: `iam.protect(communities.protected())`.
    /// Así ningún otro contexto depende de `iam` (HEX-1).
    pub fn protect(self, routes: Router) -> Router {
        routes.layer(from_fn_with_state(self, authenticate::handle))
    }
}
