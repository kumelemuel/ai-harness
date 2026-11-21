use std::time::SystemTime;

/// Reloj: capacidad técnica transversal (HEX-8).
///
/// El núcleo no lee la hora del sistema ni usa una librería de fechas: recibe
/// el instante de un `Clock`. El adaptador (`SystemClockAdapter`) vive en
/// infraestructura. El doble de test (`FixedClockAdapter`, hora fija) va en el
/// `testing.rs` del `application` que lo use, como `FixedIdGeneratorAdapter`.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
}
