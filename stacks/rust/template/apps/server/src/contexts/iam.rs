use crate::config::Config;
use iam_api::IamApi;
use iam_application::queries::verify_access_token::VerifyAccessTokenHandler;
use iam_infrastructure::JwtAccessTokenAdapter;
use kernel::FaultReporter;
use std::sync::Arc;

pub fn api(config: &Config, faults: Arc<dyn FaultReporter>) -> IamApi {
    let tokens = Arc::new(JwtAccessTokenAdapter::new(&config.jwt_secret));

    IamApi {
        verify_access_token: Arc::new(VerifyAccessTokenHandler::new(tokens)),
        faults,
    }
}
