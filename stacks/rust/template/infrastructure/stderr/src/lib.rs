//! Adaptadores de las capacidades del kernel (HEX-8) que escriben en stderr.
//!
//! Infraestructura compartida: no pertenece a ningún contexto, así que no vive
//! en un `contexts/{ctx}/infrastructure`, y solo ve el kernel (HEX-1).

mod fault_reporter;

pub use fault_reporter::StderrFaultReporterAdapter;
