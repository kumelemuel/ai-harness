//! RFC 9457 *Problem Details* (ERR-4).

use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use kernel::{Fault, FaultReporter};
use serde::Serialize;

pub const CONTENT_TYPE_PROBLEM: &str = "application/problem+json";

/// Respuesta de error. Cada contexto construye una por hoja de su contrato y le
/// agrega su `code` estable (miembro de extensión) para los clientes.
#[derive(Debug, Serialize)]
pub struct Problem {
    #[serde(rename = "type")]
    kind: &'static str,
    title: &'static str,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
}

impl Problem {
    fn new(kind: &'static str, title: &'static str, status: StatusCode) -> Self {
        Self {
            kind,
            title,
            status: status.as_u16(),
            detail: None,
            code: None,
        }
    }

    pub fn validation(detail: impl Into<String>) -> Self {
        Self::new(
            "/problems/validation",
            "Invalid request",
            StatusCode::BAD_REQUEST,
        )
        .with_detail(detail)
    }

    pub fn not_found(detail: impl Into<String>) -> Self {
        Self::new("/problems/not-found", "Not found", StatusCode::NOT_FOUND).with_detail(detail)
    }

    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::new("/problems/conflict", "Conflict", StatusCode::CONFLICT).with_detail(detail)
    }

    /// 422: la petición es válida, pero un colaborador revisó el contenido y
    /// lo rechazó (p. ej. la moderación).
    pub fn content_rejected(detail: impl Into<String>) -> Self {
        Self::new(
            "/problems/content-rejected",
            "Content rejected",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .with_detail(detail)
    }

    /// 502: un colaborador declaró que no pudo completar. Es árbol A (un error
    /// declarado del puerto), no un `Fault`: no se reporta.
    pub fn dependency_unavailable() -> Self {
        Self::new(
            "/problems/dependency-unavailable",
            "Dependency unavailable",
            StatusCode::BAD_GATEWAY,
        )
    }

    pub fn unauthorized() -> Self {
        Self::new(
            "/problems/unauthorized",
            "Unauthorized",
            StatusCode::UNAUTHORIZED,
        )
    }

    fn internal() -> Self {
        Self::new(
            "/problems/unexpected",
            "Unexpected error",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_code(mut self, code: &'static str) -> Self {
        self.code = Some(code);
        self
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_vec(&self).expect("invariante: Problem siempre serializa");
        (status, [(CONTENT_TYPE, CONTENT_TYPE_PROBLEM)], body).into_response()
    }
}

/// Manejador del árbol B: registra por el puerto y responde genérico, sin detalle.
pub fn unexpected(fault: &Fault, reporter: &dyn FaultReporter) -> Response {
    reporter.report(fault);
    Problem::internal().into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use std::sync::Mutex;

    async fn body(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn serializes_as_problem_json_with_extension_code() {
        let response = Problem::conflict("community already exists")
            .with_code("COMMUNITIES_SLUG_ALREADY_EXISTS")
            .into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(response.headers()[CONTENT_TYPE], CONTENT_TYPE_PROBLEM);
        assert_eq!(
            body(response).await,
            r#"{"type":"/problems/conflict","title":"Conflict","status":409,"detail":"community already exists","code":"COMMUNITIES_SLUG_ALREADY_EXISTS"}"#
        );
    }

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl FaultReporter for Recorder {
        fn report(&self, fault: &Fault) {
            self.0.lock().unwrap().push(fault.detail().to_string());
        }
    }

    #[tokio::test]
    async fn unexpected_reports_the_fault_and_hides_the_detail() {
        let recorder = Recorder::default();

        let response = unexpected(&Fault::new("db.internal:5432 refused"), &recorder);

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!body(response).await.contains("db.internal"));
        assert_eq!(
            *recorder.0.lock().unwrap(),
            vec!["db.internal:5432 refused"]
        );
    }
}
