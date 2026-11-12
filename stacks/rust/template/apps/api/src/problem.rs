//! RFC 9457 *Problem Details*, con un helper por hoja (ERR-4).

use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use communities_application::errors::{Conflict, Validation};
use kernel::{Fault, FaultReporter};
use serde::Serialize;

pub const CONTENT_TYPE_PROBLEM: &str = "application/problem+json";

#[derive(Debug, Serialize)]
struct Problem {
    #[serde(rename = "type")]
    kind: &'static str,
    title: &'static str,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl Problem {
    fn respond(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_vec(&self).expect("invariante: Problem siempre serializa");
        (status, [(CONTENT_TYPE, CONTENT_TYPE_PROBLEM)], body).into_response()
    }
}

pub fn validation(leaf: &Validation) -> Response {
    Problem {
        kind: "/problems/validation",
        title: "Invalid request",
        status: 400,
        detail: Some(format!("invalid field: {}", leaf.field)),
    }
    .respond()
}

pub fn conflict(leaf: &Conflict) -> Response {
    Problem {
        kind: "/problems/conflict",
        title: "Conflict",
        status: 409,
        detail: Some(format!("{} already exists", leaf.resource)),
    }
    .respond()
}

pub fn unauthorized() -> Response {
    Problem {
        kind: "/problems/unauthorized",
        title: "Unauthorized",
        status: 401,
        detail: None,
    }
    .respond()
}

/// Manejador del árbol B: registra por el puerto y responde genérico, sin detalle.
pub fn unexpected(fault: &Fault, reporter: &dyn FaultReporter) -> Response {
    reporter.report(fault);
    Problem {
        kind: "/problems/unexpected",
        title: "Unexpected error",
        status: 500,
        detail: None,
    }
    .respond()
}
