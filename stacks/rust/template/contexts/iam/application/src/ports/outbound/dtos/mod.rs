//! DTOs de los puertos salientes (HEX-6): un módulo por contrato,
//! `{concepto}_payloads.rs`, que hace de contenedor `{Contrato}Payloads`.

mod access_token_payloads;

pub use access_token_payloads::AccessTokenVerdict;
