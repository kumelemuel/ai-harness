//! Adaptador driving de billing. Cada función `bad_*` tiene un comodín sobre un
//! enum del contrato; cada `ok_*`, ninguno.
use crate::application::{CardRejection, ChargeError, Failure, Field, Invalid};

pub struct AxumInvoiceRoute;

pub fn bad_wild(error: ChargeError) -> u16 {
    match error {
        ChargeError::Declined => 402,
        _ => 400,
    }
}

pub fn bad_binding(field: Field) -> u16 {
    match field {
        Field::Amount => 1,
        _other => 0,
    }
}

pub fn bad_wrapped(result: Result<u8, ChargeError>) -> u16 {
    match result {
        Ok(_) => 200,
        Err(ChargeError::Declined) => 402,
        Err(e) => {
            let _ = e;
            400
        }
    }
}

pub fn bad_nested(failure: Failure<ChargeError>) -> u16 {
    match failure {
        Failure::Expected(ChargeError::Declined) => 402,
        Failure::Expected(_) => 400,
        Failure::Unexpected => 500,
    }
}

pub fn bad_reason(field: Field) -> u16 {
    match field {
        Field::Amount => 1,
        Field::Card(CardRejection::Expired) => 2,
        Field::Card(_) => 3,
    }
}

pub fn bad_struct_variant(error: ChargeError) -> u16 {
    match error {
        (ChargeError::Limit { .. }) => 429,
        _ => 400,
    }
}

pub fn bad_rest(leaf: &Invalid) -> u16 {
    match leaf {
        Invalid {
            field: Field::Amount,
            ..
        } => 1,
        Invalid { .. } => 0,
    }
}

pub fn bad_ref_tuple(field: &Field, hint: u8) -> u16 {
    match (field, hint) {
        (&Field::Amount, 0) => 1,
        (_, _) => 0,
    }
}

pub fn bad_if_let(error: ChargeError) -> u16 {
    if let ChargeError::Expired = error { 410 } else { 400 }
}

pub fn bad_let_else(error: ChargeError) -> u16 {
    let ChargeError::Limit { cents } = error else {
        return 400;
    };
    u16::try_from(cents).unwrap_or(u16::MAX)
}

pub fn bad_matches(error: &ChargeError) -> bool {
    matches!(error, ChargeError::Declined)
}

pub fn ok_exhaustive(result: Result<u8, Failure<ChargeError>>) -> u16 {
    match result {
        Ok(amount) => u16::from(amount),
        Err(Failure::Expected(ChargeError::Declined)) => 402,
        Err(Failure::Expected(ChargeError::Expired | ChargeError::Limit { .. })) => 400,
        Err(Failure::Expected(ChargeError::Invalid(leaf))) => ok_rest(&leaf),
        Err(Failure::Unexpected) => 500,
    }
}

/// `..` deja fuera `hint`, que no es un enum: no cuenta.
pub fn ok_rest(leaf: &Invalid) -> u16 {
    match leaf {
        Invalid {
            field: Field::Amount,
            ..
        } => 1,
        Invalid {
            field: Field::Card(reason),
            ..
        } => ok_reason(reason),
    }
}

/// Una variable que se pasa a su helper no es un comodín: no hay otro brazo
/// que nombre esa posición.
pub fn ok_reason(reason: &CardRejection) -> u16 {
    match reason {
        CardRejection::Expired => 2,
        CardRejection::Stolen => 3,
    }
}

pub fn ok_text(value: &str) -> u16 {
    if let Some(rest) = value.strip_prefix('x') { rest.len() as u16 } else { 0 }
}

pub fn ok_header(value: &str) -> u16 {
    match value {
        "json" => 1,
        _ => 0,
    }
}

#[cfg(test)]
fn test_only(error: ChargeError) -> bool {
    match error {
        ChargeError::Declined => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _any(error: ChargeError) -> bool {
        matches!(error, ChargeError::Declined)
    }
}

// hex_1_driving_adapters_are_checked_in_legacy_contexts: el dominio y un puerto
// saliente de otro contexto legacy (`iam`), por ruta y por `use`.
use iam::application::ports::outbound::PasswordHashingService as _;

pub fn owner() -> Option<iam::domain::Account> {
    None
}
