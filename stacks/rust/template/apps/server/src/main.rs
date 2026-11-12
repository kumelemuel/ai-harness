//! Composition root: el único lugar que conoce implementaciones (HEX-2).

use api::{AppState, router};
use communities_application::commands::create_community::CreateCommunityHandler;
use communities_infrastructure::{InMemoryCommunityAdapter, UuidIdGenerator};
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

#[tokio::main]
async fn main() {
    let communities = Arc::new(InMemoryCommunityAdapter::default());
    let ids = Arc::new(UuidIdGenerator);

    let state = AppState {
        create_community: Arc::new(CreateCommunityHandler::new(communities, ids)),
        faults: Arc::new(StderrFaultReporter),
    };

    // Última red del árbol B: un `panic!` (invariante roto) termina en 500.
    let app = router(state).layer(CatchPanicLayer::new());

    let address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("invariante: la dirección de escucha está disponible");

    eprintln!("listening on {address}");
    axum::serve(listener, app)
        .await
        .expect("invariante: el servidor no termina con error");
}
