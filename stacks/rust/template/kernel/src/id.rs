/// Generador de identificadores: capacidad técnica transversal (HEX-8).
///
/// El núcleo no depende de `uuid`: el adaptador (`UuidIdGeneratorAdapter`)
/// vive en infraestructura.
pub trait IdGenerator: Send + Sync {
    fn next_id(&self) -> String;
}
