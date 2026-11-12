/// Generador de identificadores (helper genérico, HEX-8).
///
/// El núcleo no depende de `uuid`: la implementación vive en infraestructura.
pub trait IdGenerator: Send + Sync {
    fn next_id(&self) -> String;
}
