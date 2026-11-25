pub mod dtos;

/// Puerto saliente con rol `Service`: es el uso permitido del sufijo (HEX-7).
///
/// hex_6_outbound_port_types_are_checked_in_legacy_contexts espera dos
/// violaciones: `HashedPassword` y el alias `Salt`, definidos junto al puerto.
/// `dtos::HashRequest` está en su lugar, y `crate::domain::Account` es del
/// dominio aunque application tenga un homónimo.
pub trait PasswordHashingService {
    fn hash(&self, plain: &str) -> String;
    fn hash_request(&self, request: dtos::HashRequest) -> HashedPassword;
    fn salt(&self) -> Salt;
    fn owner(&self) -> Option<crate::domain::Account>;
}

pub struct HashedPassword;

pub type Salt = String;
