//! Hojas de error compartidas del contexto (ERR-2).
//!
//! Viven en `application` porque `api` solo ve `application` (HEX-1). Cada
//! contrato `{Verb}{Noun}Error` declara qué hojas admite con una variante por hoja.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validation {
    pub field: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub resource: &'static str,
}

/// Un colaborador revisó el contenido y lo rechazó (p. ej. la moderación).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRejected {
    pub field: &'static str,
    pub reason: String,
}

/// Un colaborador declaró que no pudo completar (ERR-3): no disponible, timeout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyUnavailable {
    pub dependency: &'static str,
}
