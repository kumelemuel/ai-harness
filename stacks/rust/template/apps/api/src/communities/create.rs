use crate::{AppState, problem};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use communities_application::commands::create_community::{
    CreateCommunityCommand, CreateCommunityError, CreateCommunityResponse,
};
use kernel::Failure;
use serde::{Deserialize, Serialize};

/// Vocabulario de transporte (HEX-6): `{Caso}HttpRequest` / `{Caso}HttpResponse`.
#[derive(Debug, Deserialize)]
pub struct CreateCommunityHttpRequest {
    pub slug: String,
    pub name: String,
    pub is_public: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateCommunityHttpResponse {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub is_public: bool,
}

impl From<CreateCommunityResponse> for CreateCommunityHttpResponse {
    fn from(response: CreateCommunityResponse) -> Self {
        Self {
            id: response.id,
            slug: response.slug,
            name: response.name,
            is_public: response.is_public,
        }
    }
}

/// En un proyecto real la identidad la resuelve un middleware de autenticación.
const ACCOUNT_HEADER: &str = "x-account-id";

pub async fn create_community(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateCommunityHttpRequest>,
) -> Response {
    let Some(owner_id) = headers.get(ACCOUNT_HEADER).and_then(|v| v.to_str().ok()) else {
        return problem::unauthorized();
    };

    let command = CreateCommunityCommand {
        owner_id: owner_id.to_string(),
        slug: body.slug,
        name: body.name,
        is_public: body.is_public,
    };

    // `fold` exhaustivo del contrato hacia el transporte (ERR-4).
    match state.create_community.execute(command).await {
        Ok(response) => (
            StatusCode::CREATED,
            Json(CreateCommunityHttpResponse::from(response)),
        )
            .into_response(),
        Err(Failure::Expected(error)) => match error {
            CreateCommunityError::Validation(leaf) => problem::validation(&leaf),
            CreateCommunityError::Conflict(leaf) => problem::conflict(&leaf),
        },
        Err(Failure::Unexpected(fault)) => problem::unexpected(&fault, state.faults.as_ref()),
    }
}
