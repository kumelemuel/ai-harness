use kernel::ValueObject;

/// Identidad del dueño, propia de este contexto.
///
/// No se importa el `AccountId` de otro contexto: cada contexto modela su
/// propia vista del concepto.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnerId(String);

impl OwnerId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ValueObject for OwnerId {}
