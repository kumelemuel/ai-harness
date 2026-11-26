//! hex_2_core_imports_are_checked_in_legacy_contexts: el manifiesto del
//! contexto declara `axum`, así que Cargo no impide nombrarlo en el dominio.
//! Una violación por crate externo: el `use` y la ruta completa cuentan una vez.
//! El parámetro `axum` es una variable, no el crate.
use axum::Json;

pub fn wrap(axum: u8) -> Json<u8> {
    let _ = axum::http::StatusCode::OK;
    Json(axum)
}
