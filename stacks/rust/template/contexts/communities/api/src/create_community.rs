use crate::{CommunitiesApi, codes};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use communities_application::commands::create_community::{
    CreateCommunityCommand, CreateCommunityError, CreateCommunityResponse,
};
use communities_application::errors::{Conflict, Validation};
use edge_http::{AuthenticatedAccount, Problem, unexpected};
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

pub(crate) async fn handle(
    State(api): State<CommunitiesApi>,
    identity: Option<Extension<AuthenticatedAccount>>,
    Json(body): Json<CreateCommunityHttpRequest>,
) -> Response {
    let Some(Extension(identity)) = identity else {
        return Problem::unauthorized().into_response();
    };

    let command = CreateCommunityCommand {
        owner_id: identity.account_id,
        slug: body.slug,
        name: body.name,
        is_public: body.is_public,
    };

    // `fold` exhaustivo del contrato hacia el transporte (ERR-4).
    match api.create_community.execute(command).await {
        Ok(response) => (
            StatusCode::CREATED,
            Json(CreateCommunityHttpResponse::from(response)),
        )
            .into_response(),
        Err(Failure::Expected(error)) => match error {
            CreateCommunityError::Validation(leaf) => validation(&leaf),
            CreateCommunityError::Conflict(leaf) => conflict(&leaf),
        }
        .into_response(),
        Err(Failure::Unexpected(fault)) => unexpected(&fault, api.faults.as_ref()),
    }
}

/// Un helper por hoja (ERR-4).
fn validation(leaf: &Validation) -> Problem {
    let code = match leaf.field {
        "slug" => codes::INVALID_COMMUNITY_SLUG,
        "name" => codes::INVALID_COMMUNITY_NAME,
        _ => codes::INVALID_REQUEST,
    };
    Problem::validation(format!("invalid field: {}", leaf.field)).with_code(code)
}

fn conflict(leaf: &Conflict) -> Problem {
    Problem::conflict(format!("{} already exists", leaf.resource))
        .with_code(codes::SLUG_ALREADY_EXISTS)
}
