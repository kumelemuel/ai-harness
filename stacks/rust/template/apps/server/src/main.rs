//! Composition root: el único lugar que conoce implementaciones (HEX-2).
//! Construye cada contexto y compone sus rutas.

use axum::Router;
use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use communities_api::CommunitiesApi;
use communities_application::commands::create_community::CreateCommunityHandler;
use communities_infrastructure::{
    InMemoryCommunityAdapter, NoOpModerationAdapter, UuidIdGeneratorAdapter,
};
use edge_http::AuthenticatedAccount;
use kernel::{Fault, FaultReporter};
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;

/// Registro del árbol B. En producción: tracing, Sentry, etc.
struct StderrFaultReporter;

impl FaultReporter for StderrFaultReporter {
    fn report(&self, fault: &Fault) {
        eprintln!("unexpected failure: {:?}", fault.detail());
    }
}

/// Autenticación de ejemplo: en producción, verificar un token y derivar la cuenta.
async fn authenticate(mut request: Request, next: Next) -> Response {
    let account = request
        .headers()
        .get("x-account-id")
        .and_then(|value| value.to_str().ok())
        .map(|account_id| AuthenticatedAccount {
            account_id: account_id.to_string(),
        });
    if let Some(account) = account {
        request.extensions_mut().insert(account);
    }
    next.run(request).await
}

fn communities(faults: Arc<dyn FaultReporter>) -> CommunitiesApi {
    let repository = Arc::new(InMemoryCommunityAdapter::default());
    // Sin servicio de moderación real todavía: aprueba todo.
    let moderation = Arc::new(NoOpModerationAdapter);
    let ids = Arc::new(UuidIdGeneratorAdapter);

    CommunitiesApi {
        create_community: Arc::new(CreateCommunityHandler::new(repository, moderation, ids)),
        faults,
    }
}

#[tokio::main]
async fn main() {
    let faults: Arc<dyn FaultReporter> = Arc::new(StderrFaultReporter);

    let protected = Router::new()
        .merge(communities(faults.clone()).protected())
        .layer(middleware::from_fn(authenticate));

    // Última red del árbol B: un `panic!` (invariante roto) termina en 500.
    let app = Router::new().merge(protected).layer(CatchPanicLayer::new());

    let address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("invariante: la dirección de escucha está disponible");

    eprintln!("listening on {address}");
    axum::serve(listener, app)
        .await
        .expect("invariante: el servidor no termina con error");
}
