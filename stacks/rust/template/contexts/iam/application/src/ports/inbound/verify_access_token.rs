use crate::queries::verify_access_token::{
    VerifyAccessTokenError, VerifyAccessTokenQuery, VerifyAccessTokenResponse,
};
use kernel::{BoxFuture, Failure};

/// Puerto entrante (HEX-4): verifica un token de acceso y devuelve la cuenta.
///
/// Lo invoca el middleware de autenticación de `iam-api`, que el composition
/// root aplica a las rutas protegidas de todos los contextos.
pub trait VerifyAccessTokenUseCase: Send + Sync {
    fn execute(
        &self,
        query: VerifyAccessTokenQuery,
    ) -> BoxFuture<'_, Result<VerifyAccessTokenResponse, Failure<VerifyAccessTokenError>>>;
}
