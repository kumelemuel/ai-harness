//! Adaptadores driven del contexto `iam` (HEX-5: `{Tecnología}{Concepto}Adapter`).

mod jwt_access_token;

pub use jwt_access_token::JwtAccessTokenAdapter;
