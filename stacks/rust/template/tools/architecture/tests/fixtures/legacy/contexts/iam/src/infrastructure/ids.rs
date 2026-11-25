// Capacidad del kernel implementada dentro de un contexto legacy, con alias:
// hex_8_capability_adapters_are_checked_in_legacy_contexts la espera.
use shared::IdGenerator as Ids;

pub struct UuidIdGeneratorAdapter;

impl Ids for UuidIdGeneratorAdapter {
    fn next_id(&self) -> String {
        String::new()
    }
}
