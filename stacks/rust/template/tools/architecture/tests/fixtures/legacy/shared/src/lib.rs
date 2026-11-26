//! Kernel legacy.

/// Dominio del kernel legacy (`shared::domain`): para los contratos es dominio.
pub mod domain {
    pub struct VerifiedIdentity;
}

/// hex_2_core_imports_are_checked_in_legacy_contexts: el kernel legacy también.
pub fn shape(_: serde::de::IgnoredAny) {}
