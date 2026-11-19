//! Hojas de error compartidas del contexto (ERR-2).

/// La petición no trae una identidad verificable. No dice por qué (token
/// ausente, vencido o manipulado): el motivo no se le entrega al cliente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unauthenticated;
