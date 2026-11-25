use kernel::IdGenerator;

/// UUID v4 en texto (HEX-5: `{Tecnología}{Concepto}Adapter`).
pub struct UuidIdGeneratorAdapter;

impl IdGenerator for UuidIdGeneratorAdapter {
    fn next_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// UUID v4 en texto: 36 caracteres, guiones en 8-4-4-4-12, versión 4 y
    /// variante RFC 4122 (`8`, `9`, `a` o `b`).
    fn is_uuid_v4(id: &str) -> bool {
        let parts: Vec<&str> = id.split('-').collect();
        let lengths: Vec<usize> = parts.iter().map(|p| p.len()).collect();
        lengths == [8, 4, 4, 4, 12]
            && id.chars().all(|c| c == '-' || c.is_ascii_hexdigit())
            && parts[2].starts_with('4')
            && parts[3].starts_with(['8', '9', 'a', 'b'])
    }

    #[test]
    fn next_id_is_a_uuid_v4() {
        let id = UuidIdGeneratorAdapter.next_id();

        assert!(is_uuid_v4(&id), "{id}");
    }

    #[test]
    fn next_id_does_not_repeat() {
        let ids: BTreeSet<String> = (0..100).map(|_| UuidIdGeneratorAdapter.next_id()).collect();

        assert_eq!(ids.len(), 100);
    }
}
