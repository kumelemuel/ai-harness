//! hex_2_core_imports_name_each_external_crate_once: una violación por crate
//! externo nombrado, de cualquier forma; ninguna por lo local ni lo interno.

// `use` de un crate, con alias y en grupo: un crate distinto por forma.
use either;
use rand as r;
use {itertools::Itertools as _, crate::money::Amount};
// `extern crate`, con alias.
extern crate chrono as c;

// Interna renombrada (`k` es el kernel): no cuenta.
use k::Fault;

// Local con el nombre de un crate declarado (`money`): no cuenta.
pub mod money {
    pub struct Amount;
    impl Amount {
        pub fn zero() -> Self {
            Amount
        }
    }
}

// Dentro de `derive`, que `syn` deja como tokens.
#[derive(serde::Serialize)]
pub struct Order;

pub fn total(uuid: u8) -> u8 {
    // Dentro del cuerpo de una macro.
    let _ids = vec![uuid::Uuid::nil()];
    // Renombre del workspace: `json` es `serde_json`.
    let _value = json::Value::Null;
    let _amount = money::Amount::zero();
    let _fault: Option<Fault> = None;
    let _ = Amount::zero();
    // Ruta con `::` inicial.
    ::log::info!("total");
    // Variable con el nombre de un crate: no es una ruta.
    uuid
}
