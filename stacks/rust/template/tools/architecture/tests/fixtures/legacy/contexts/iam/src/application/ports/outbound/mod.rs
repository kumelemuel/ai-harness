pub mod dtos;

/// Puerto saliente con rol `Service`: es el uso permitido del sufijo (HEX-7).
///
/// `HashedPassword` está definido junto al puerto, fuera de `dtos/`:
/// hex_6_outbound_port_types_are_checked_in_legacy_contexts espera la violación.
pub trait PasswordHashingService {
    fn hash(&self, plain: &str) -> String;
    fn hash_request(&self, request: dtos::HashRequest) -> HashedPassword;
}

pub struct HashedPassword;
