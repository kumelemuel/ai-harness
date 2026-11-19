#![allow(clippy::unwrap_used)] // crate de test: un unwrap fallido es el test fallando.

//! Tests del middleware (TDD-2) con un caso de uso stub: `api` se prueba sin
//! dominio ni infraestructura.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header::AUTHORIZATION, header::CONTENT_TYPE};
use axum::routing::get;
use axum::{Extension, Router};
use edge_http::{AuthenticatedAccount, CONTENT_TYPE_PROBLEM};
use iam_api::IamApi;
use iam_application::errors::Unauthenticated;
use iam_application::ports::inbound::VerifyAccessTokenUseCase;
use iam_application::queries::verify_access_token::{
    VerifyAccessTokenError, VerifyAccessTokenQuery, VerifyAccessTokenResponse,
};
use kernel::{BoxFuture, Failure, Fault, FaultReporter};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

type Outcome = Result<VerifyAccessTokenResponse, Failure<VerifyAccessTokenError>>;

/// Responde fijo y registra los tokens que recibió.
struct StubUseCase {
    outcome: fn() -> Outcome,
    tokens: Mutex<Vec<String>>,
}

impl VerifyAccessTokenUseCase for StubUseCase {
    fn execute(&self, query: VerifyAccessTokenQuery) -> BoxFuture<'_, Outcome> {
        self.tokens.lock().unwrap().push(query.token);
        let outcome = (self.outcome)();
        Box::pin(async move { outcome })
    }
}

#[derive(Default)]
struct SpyFaultReporterAdapter(Mutex<Vec<String>>);

impl FaultReporter for SpyFaultReporterAdapter {
    fn report(&self, fault: &Fault) {
        self.0.lock().unwrap().push(fault.detail().to_string());
    }
}

struct Sent {
    status: StatusCode,
    content_type: String,
    body: String,
    tokens: Vec<String>,
    faults: Vec<String>,
}

/// Una ruta protegida que devuelve la cuenta que le entregó el middleware.
async fn get_protected(outcome: fn() -> Outcome, authorization: Option<&str>) -> Sent {
    let use_case = Arc::new(StubUseCase {
        outcome,
        tokens: Mutex::new(Vec::new()),
    });
    let reporter = Arc::new(SpyFaultReporterAdapter::default());
    let routes = Router::new().route(
        "/me",
        get(
            |Extension(account): Extension<AuthenticatedAccount>| async move { account.account_id },
        ),
    );
    let app = IamApi {
        verify_access_token: use_case.clone(),
        faults: reporter.clone(),
    }
    .protect(routes);

    let mut request = Request::get("/me");
    if let Some(value) = authorization {
        request = request.header(AUTHORIZATION, value);
    }
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();

    Sent {
        status,
        content_type,
        body: String::from_utf8(bytes.to_vec()).unwrap(),
        tokens: use_case.tokens.lock().unwrap().clone(),
        faults: reporter.0.lock().unwrap().clone(),
    }
}

fn alice() -> Outcome {
    Ok(VerifyAccessTokenResponse {
        account_id: "alice".into(),
    })
}

fn rejected() -> Outcome {
    Err(Failure::expected(Unauthenticated))
}

fn outage() -> Outcome {
    Err(Failure::Unexpected(Fault::new("keystore.internal refused")))
}

#[tokio::test]
async fn a_verified_token_reaches_the_route_with_its_account() {
    let sent = get_protected(alice, Some("Bearer abc.def.ghi")).await;

    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(sent.body, "alice");
    assert_eq!(sent.tokens, vec!["abc.def.ghi".to_string()]);
}

#[tokio::test]
async fn without_authorization_it_is_401_without_verifying() {
    let sent = get_protected(alice, None).await;

    assert_eq!(sent.status, StatusCode::UNAUTHORIZED);
    assert_eq!(sent.content_type, CONTENT_TYPE_PROBLEM);
    assert!(
        sent.body.contains(r#""code":"IAM_UNAUTHENTICATED""#),
        "{}",
        sent.body
    );
    assert!(sent.tokens.is_empty());
}

#[tokio::test]
async fn a_non_bearer_scheme_is_401_without_verifying() {
    let sent = get_protected(alice, Some("Basic YWxpY2U6c2VjcmV0")).await;

    assert_eq!(sent.status, StatusCode::UNAUTHORIZED);
    assert!(sent.tokens.is_empty());
}

#[tokio::test]
async fn an_empty_bearer_is_401_without_verifying() {
    let sent = get_protected(alice, Some("Bearer   ")).await;

    assert_eq!(sent.status, StatusCode::UNAUTHORIZED);
    assert!(sent.tokens.is_empty());
}

#[tokio::test]
async fn unauthenticated_is_401_problem_with_code() {
    let sent = get_protected(rejected, Some("Bearer expired")).await;

    assert_eq!(sent.status, StatusCode::UNAUTHORIZED);
    assert_eq!(sent.content_type, CONTENT_TYPE_PROBLEM);
    assert!(
        sent.body.contains(r#""code":"IAM_UNAUTHENTICATED""#),
        "{}",
        sent.body
    );
    assert!(sent.faults.is_empty(), "árbol A: no se reporta");
}

#[tokio::test]
async fn a_fault_is_500_reported_and_without_detail() {
    let sent = get_protected(outage, Some("Bearer abc")).await;

    assert_eq!(sent.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!sent.body.contains("keystore.internal"));
    assert_eq!(sent.faults, vec!["keystore.internal refused".to_string()]);
}
