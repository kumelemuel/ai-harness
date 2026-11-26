//! Adaptador driving (Api): `infrastructure/http` se asume driving.
//!
//! hex_1_driving_adapters_are_checked_in_legacy_contexts espera exactamente
//! cuatro violaciones: el dominio (por `use` y por ruta relativa) y un puerto
//! saliente (por `use` y por el módulo `ports` importado). El puerto entrante y
//! el módulo de test no cuentan.
use crate::application::ports;
use crate::application::ports::inbound::RegisterAccountUseCase;
use crate::application::ports::outbound::PasswordHashingService;
use crate::domain::Account;
// Api: puede nombrar `axum` (hex_2_core_imports_… no lo cuenta).
use axum::Json as _;

pub struct AxumRegisterAccountRoute;

pub fn register(
    _use_case: &dyn RegisterAccountUseCase,
    _hasher: &dyn PasswordHashingService,
    _again: &dyn ports::outbound::PasswordHashingService,
) -> Option<Account> {
    None::<super::super::domain::Account>
}

#[cfg(test)]
mod tests {
    use crate::domain::Account;

    fn _fixture(_a: Account) {}
}
