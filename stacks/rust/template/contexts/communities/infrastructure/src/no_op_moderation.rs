use communities_application::ports::outbound::ModerationGateway;
use communities_application::ports::outbound::dtos::{
    ModerationError, ModerationVerdict, ReviewContentRequest,
};
use kernel::{BoxFuture, Failure};
use std::future::ready;

/// Adaptador de `ModerationGateway` que aprueba todo sin consultar a nadie.
///
/// Es el que cablea el composition root mientras no haya un servicio de
/// moderación real (`HttpModerationAdapter`).
pub struct NoOpModerationAdapter;

impl ModerationGateway for NoOpModerationAdapter {
    fn review_content(
        &self,
        _request: ReviewContentRequest,
    ) -> BoxFuture<'_, Result<ModerationVerdict, Failure<ModerationError>>> {
        Box::pin(ready(Ok(ModerationVerdict::Approved)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use communities_application::testing::block_on;

    communities_application::moderation_gateway_contract! {
        new_adapter: |_| NoOpModerationAdapter,
        run: block_on,
        approves: supported,
        // El NoOp no consulta a ningún servicio: no puede rechazar ni dejar de
        // responder. Se declara aquí, con su motivo, en vez de saltarse el caso.
        rejects: unsupported("NoOp aprueba todo: no consulta un servicio que pueda rechazar"),
        unavailable: unsupported("NoOp no hace E/S: no hay servicio que pueda no responder"),
    }
}
