/// Generador de identificadores: capacidad técnica transversal (HEX-8).
///
/// El núcleo no depende de `uuid`: el adaptador (`UuidIdGeneratorAdapter`)
/// vive en la infraestructura compartida (`infrastructure-uuid`), nunca en un
/// contexto.
pub trait IdGenerator: Send + Sync {
    fn next_id(&self) -> String;
}
