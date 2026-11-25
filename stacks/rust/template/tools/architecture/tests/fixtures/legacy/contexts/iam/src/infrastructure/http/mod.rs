//! Adaptador driving (Api): `infrastructure/http` se asume driving.
//!
//! Importa el dominio y un puerto saliente: hex_1_driving_adapters_are_checked_
//! in_legacy_contexts espera las dos violaciones.
use crate::application::ports::outbound::PasswordHashingService;
use crate::domain::Account;

pub struct AxumRegisterAccountRoute;

pub fn register(_hasher: &dyn PasswordHashingService) -> Option<Account> {
    None
}
