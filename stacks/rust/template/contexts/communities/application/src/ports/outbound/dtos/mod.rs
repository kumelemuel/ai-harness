//! DTOs de los puertos salientes (HEX-6): un módulo por contrato,
//! `{concepto}_payloads.rs`, que hace de contenedor `{Contrato}Payloads`.
//! Entrada `…Request`; salida con el nombre del concepto, sin sufijo.

mod moderation_payloads;

pub use moderation_payloads::{ModerationError, ModerationVerdict, ReviewContentRequest};
