//! Adaptador de `IdGenerator` (HEX-8) con UUID v4.
//!
//! Infraestructura compartida: todos los contextos generan ids con la misma
//! capacidad, así que su adaptador no vive en un `contexts/{ctx}/infrastructure`
//! (`hex_8_capability_adapters_are_shared_infrastructure`). `uuid` vive aquí,
//! nunca en el núcleo (HEX-2).

mod id_generator;

pub use id_generator::UuidIdGeneratorAdapter;
