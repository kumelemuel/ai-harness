//! Contrato de billing: err_4_driving_folds_are_checked_in_legacy_contexts lo
//! traduce en `infrastructure/http.rs`.
pub enum ChargeError {
    Declined,
    Expired,
    Limit { cents: u32 },
    Invalid(Invalid),
}

/// Como `kernel::Failure`: el contrato viaja dentro.
pub enum Failure<E> {
    Expected(E),
    Unexpected,
}

/// Campo que el dominio rechazó: cerrado, como el `Field` de la plantilla. El
/// de `Card` lleva su motivo.
pub enum Field {
    Amount,
    Card(CardRejection),
}

pub enum CardRejection {
    Expired,
    Stolen,
}

/// Hoja con un campo cerrado y otro que no lo es.
pub struct Invalid {
    pub field: Field,
    pub hint: u8,
}
