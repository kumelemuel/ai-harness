use std::time::SystemTime;

/// Reloj: capacidad técnica transversal (HEX-8).
///
/// El núcleo no lee la hora del sistema ni usa una librería de fechas: recibe
/// el instante de un `Clock`. El adaptador (`SystemClockAdapter`) vive en
/// infraestructura, y un test usa un doble con la hora fija.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
}
