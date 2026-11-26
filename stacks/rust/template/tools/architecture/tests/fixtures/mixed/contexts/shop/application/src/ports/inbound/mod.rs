//! hex_4_a_unit_response_is_one_violation: casos de uso sin datos que
//! responden `()` en vez de un `{Verb}{Noun}Response` vacío, directo y en un
//! `BoxFuture` (la forma de la plantilla).
use crate::commands::ship_order::ShipOrderCommand;
use crate::commands::verify_order::VerifyOrderCommand;
use kernel::{BoxFuture, Fault};

pub trait VerifyOrderUseCase {
    fn execute(&self, command: VerifyOrderCommand) -> Result<(), Fault>;
}

pub trait ShipOrderUseCase {
    fn execute(&self, command: ShipOrderCommand) -> BoxFuture<'_, Result<(), Fault>>;
}
