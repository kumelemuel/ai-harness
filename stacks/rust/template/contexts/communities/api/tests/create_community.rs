#![allow(clippy::unwrap_used)] // crate de test: un unwrap fallido es el test fallando.

//! Tests del endpoint (TDD-2) con un caso de uso stub: `api` se prueba sin
//! dominio ni infraestructura.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header::CONTENT_TYPE};
use communities_api::CommunitiesApi;
use communities_application::commands::create_community::{
    CreateCommunityCommand, CreateCommunityError, CreateCommunityResponse,
};
use communities_application::errors::{Conflict, ContentRejected, DependencyUnavailable};
use communities_application::ports::inbound::CreateCommunityUseCase;
use edge_http::AuthenticatedAccount;
use kernel::{BoxFuture, Failure, Fault, FaultReporter};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

type Outcome = Result<CreateCommunityResponse, Failure<CreateCommunityError>>;

struct StubUseCase(fn() -> Outcome);

impl CreateCommunityUseCase for StubUseCase {
    fn execute(&self, _: CreateCommunityCommand) -> BoxFuture<'_, Outcome> {
        let outcome = (self.0)();
        Box::pin(async move { outcome })
    }
}

#[derive(Default)]
struct RecordingReporter(Mutex<Vec<String>>);

impl FaultReporter for RecordingReporter {
    fn report(&self, fault: &Fault) {
        self.0.lock().unwrap().push(fault.detail().to_string());
    }
}

async fn post(
    outcome: fn() -> Outcome,
    account: Option<&str>,
) -> (StatusCode, String, String, Arc<RecordingReporter>) {
    let reporter = Arc::new(RecordingReporter::default());
    let app = CommunitiesApi {
        create_community: Arc::new(StubUseCase(outcome)),
        faults: reporter.clone(),
    }
    .protected();

    let mut request = Request::post("/communities").header(CONTENT_TYPE, "application/json");
    if let Some(account) = account {
        // En producción la inserta el middleware de autenticación del composition root.
        request = request.extension(AuthenticatedAccount {
            account_id: account.into(),
        });
    }
    let body = Body::from(r#"{"slug":"rust-chile","name":"Rust Chile","is_public":true}"#);

    let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let content_type = response.headers()[CONTENT_TYPE]
        .to_str()
        .unwrap()
        .to_string();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();

    (
        status,
        content_type,
        String::from_utf8(bytes.to_vec()).unwrap(),
        reporter,
    )
}

#[tokio::test]
async fn created_returns_201_with_http_response() {
    let (status, _, body, _) = post(
        || {
            Ok(CreateCommunityResponse {
                id: "c-1".into(),
                slug: "rust-chile".into(),
                name: "Rust Chile".into(),
                is_public: true,
            })
        },
        Some("alice"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.contains(r#""id":"c-1""#), "{body}");
}

#[tokio::test]
async fn conflict_returns_409_problem_details() {
    let (status, content_type, body, _) = post(
        || {
            Err(Failure::expected(Conflict {
                resource: "community",
            }))
        },
        Some("alice"),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(content_type, "application/problem+json");
    assert!(body.contains(r#""type":"/problems/conflict""#), "{body}");
    assert!(
        body.contains(r#""code":"COMMUNITIES_SLUG_ALREADY_EXISTS""#),
        "{body}"
    );
}

#[tokio::test]
async fn content_rejected_returns_422_problem_details() {
    let (status, content_type, body, _) = post(
        || {
            Err(Failure::expected(ContentRejected {
                field: "name",
                reason: "offensive language".into(),
            }))
        },
        Some("alice"),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(content_type, "application/problem+json");
    assert!(
        body.contains(r#""type":"/problems/content-rejected""#),
        "{body}"
    );
    assert!(
        body.contains(r#""code":"COMMUNITIES_CONTENT_REJECTED""#),
        "{body}"
    );
    assert!(body.contains("offensive language"), "{body}");
}

#[tokio::test]
async fn dependency_unavailable_returns_502_problem_details_without_reporting() {
    let (status, content_type, body, reporter) = post(
        || {
            Err(Failure::expected(DependencyUnavailable {
                dependency: "moderation",
            }))
        },
        Some("alice"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(content_type, "application/problem+json");
    assert!(
        body.contains(r#""type":"/problems/dependency-unavailable""#),
        "{body}"
    );
    assert!(
        body.contains(r#""code":"COMMUNITIES_DEPENDENCY_UNAVAILABLE""#),
        "{body}"
    );
    // Árbol A: se declaró en la firma del puerto; no es un `Fault` que registrar.
    assert!(reporter.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn fault_returns_generic_500_and_is_reported_without_leaking() {
    let (status, content_type, body, reporter) = post(
        || Err(Failure::Unexpected(Fault::new("db.internal:5432 refused"))),
        Some("alice"),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(content_type, "application/problem+json");
    assert!(
        !body.contains("db.internal"),
        "el detalle no sale al cliente: {body}"
    );
    assert_eq!(
        *reporter.0.lock().unwrap(),
        vec!["db.internal:5432 refused".to_string()]
    );
}

#[tokio::test]
async fn missing_identity_returns_401() {
    let (status, _, _, _) = post(|| unreachable!("no debe invocarse el caso de uso"), None).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
