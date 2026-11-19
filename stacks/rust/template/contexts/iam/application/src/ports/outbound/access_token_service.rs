use super::dtos::AccessTokenVerdict;
use kernel::{BoxFuture, Fault};

/// Puerto saliente que verifica tokens de acceso (HEX-5). Es `Service`, el rol
/// de último recurso: verificar una firma es una utilidad técnica local, no
/// persistencia ni un tercero.
///
/// Semántica (ERR-3): un token inválido (mal formado, vencido o con otra
/// firma) es una respuesta del verificador, no un error: viaja en
/// `AccessTokenVerdict::Invalid`. No declara errores; cualquier falla técnica
/// es `Fault` (árbol B).
pub trait AccessTokenService: Send + Sync {
    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<AccessTokenVerdict, Fault>>;
}
