use kernel::IdGenerator;

/// Implementación de `IdGenerator`. `uuid` vive aquí, nunca en el núcleo (HEX-2).
pub struct UuidIdGenerator;

impl IdGenerator for UuidIdGenerator {
    fn next_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
