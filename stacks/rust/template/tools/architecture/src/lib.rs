//! Tests de arquitectura del workspace (VER-1). Ver `tests/`.
//!
//! - `dependencies.rs`: dirección de dependencias entre crates (HEX-1, HEX-2).
//! - `naming.rs`: nombres y vocabulario (HEX-4, HEX-5, HEX-6).
//! - `ports.rs`: el dominio no declara puertos (HEX-3).
//! - `errors.rs`: ni el dominio ni los errores de `application` implementan `Error` (ERR-1).
//! - `output.rs`: el núcleo no escribe en la salida (ERR-4).
//! - `kernel.rs`: kernel delgado, solo marcadores y capacidades de la lista (HEX-8).
