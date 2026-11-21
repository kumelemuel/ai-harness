//! Adaptadores de las capacidades del kernel (HEX-8) que leen el sistema.
//!
//! Infraestructura compartida: no pertenece a ningún contexto, así que no vive
//! en un `contexts/{ctx}/infrastructure`, y solo ve el kernel (HEX-1).

mod clock;

pub use clock::SystemClockAdapter;
