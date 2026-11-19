use crate::{IamApi, codes};
use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use edge_http::{AuthenticatedAccount, Problem, unexpected};
use iam_application::errors::Unauthenticated;
use iam_application::queries::verify_access_token::{
    VerifyAccessTokenError, VerifyAccessTokenQuery,
};
use kernel::Failure;

pub(crate) async fn handle(
    State(api): State<IamApi>,
    mut request: Request,
    next: Next,
) -> Response {
    // Forma del transporte (HEX-7): sin cabecera `Bearer` no hay nada que verificar.
    let Some(token) = bearer_token(&request) else {
        return unauthenticated(&Unauthenticated).into_response();
    };
    let query = VerifyAccessTokenQuery { token };

    // `match` exhaustivo del contrato hacia el transporte (ERR-4).
    match api.verify_access_token.execute(query).await {
        Ok(response) => {
            request.extensions_mut().insert(AuthenticatedAccount {
                account_id: response.account_id,
            });
            next.run(request).await
        }
        Err(Failure::Expected(error)) => match error {
            VerifyAccessTokenError::Unauthenticated(leaf) => unauthenticated(&leaf),
        }
        .into_response(),
        Err(Failure::Unexpected(fault)) => unexpected(&fault, api.faults.as_ref()),
    }
}

fn bearer_token(request: &Request) -> Option<String> {
    let value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?.trim();
    (!token.is_empty()).then(|| token.to_string())
}

/// Un helper por hoja (ERR-4). Sin detalle: no se dice si faltaba, venció o
/// estaba alterado.
fn unauthenticated(_leaf: &Unauthenticated) -> Problem {
    Problem::unauthorized().with_code(codes::UNAUTHENTICATED)
}
