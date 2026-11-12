use crate::commands::create_community::{
    CreateCommunityCommand, CreateCommunityError, CreateCommunityResponse,
};
use kernel::{BoxFuture, Failure};

/// Puerto entrante (HEX-4): un caso de uso, un método.
pub trait CreateCommunityUseCase: Send + Sync {
    fn execute(
        &self,
        command: CreateCommunityCommand,
    ) -> BoxFuture<'_, Result<CreateCommunityResponse, Failure<CreateCommunityError>>>;
}
