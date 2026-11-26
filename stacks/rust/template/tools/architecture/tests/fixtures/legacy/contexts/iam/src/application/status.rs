//! hex_2_core_imports_are_checked_in_legacy_contexts: una ruta completa, sin
//! `use`, también nombra el crate.
pub fn ok() -> u16 {
    ::axum::http::StatusCode::OK.as_u16()
}
