//! Composition root: el único lugar que conoce implementaciones (HEX-2).
//!
//! Cablea, no implementa: lee la configuración (`config`), construye cada
//! contexto en su módulo (`contexts/{ctx}.rs`) y compone sus rutas aquí.

mod config;
mod contexts;

use axum::Router;
use config::Config;
use infrastructure_stderr::StderrFaultReporterAdapter;
use kernel::FaultReporter;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;

#[tokio::main]
async fn main() {
    let config = Config::from_lookup(|name| std::env::var(name).ok()).unwrap_or_else(|error| {
        eprintln!("configuración inválida: {error}");
        std::process::exit(1);
    });
    let faults: Arc<dyn FaultReporter> = Arc::new(StderrFaultReporterAdapter);

    let iam = contexts::iam::api(&config, faults.clone());
    let communities = contexts::communities::api(faults);

    // `iam` protege las rutas de los demás contextos sin que dependan de él.
    let protected = iam.protect(communities.protected());

    // Última red del árbol B: un `panic!` (invariante roto) termina en 500.
    let app = Router::new().merge(protected).layer(CatchPanicLayer::new());

    let listener = tokio::net::TcpListener::bind(&config.bind_address)
        .await
        .expect("invariante: la dirección de escucha está disponible");

    eprintln!("listening on {}", config.bind_address);
    axum::serve(listener, app)
        .await
        .expect("invariante: el servidor no termina con error");
}
