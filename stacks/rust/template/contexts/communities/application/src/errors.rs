//! Hojas de error compartidas del contexto (ERR-2).
//!
//! Viven en `application` porque `api` solo ve `application` (HEX-1). Cada
//! contrato `{Verb}{Noun}Error` declara qué hojas admite con una variante por hoja.

/// Campo de la petición que el dominio rechazó. Cerrado: el borde le da su
/// código con un `match` que nombra cada campo, sin comodín (ERR-4), y un campo
/// nuevo no compila hasta tener el suyo. Si un campo se rechaza por varios
/// motivos, cada uno con su código, la variante lleva el motivo
/// (`Owner(OwnerRejection)`, con `enum OwnerRejection { Nil, WrongFormat }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Slug,
    Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validation {
    pub field: Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub resource: &'static str,
}

/// Un colaborador revisó el contenido y lo rechazó (p. ej. la moderación).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRejected {
    pub field: Field,
    pub reason: String,
}

/// Un colaborador declaró que no pudo completar (ERR-3): no disponible, timeout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyUnavailable {
    pub dependency: &'static str,
}
