use crate::application::ports::inbound::AccountEventSubscriber;

/// Suscriptor bien ubicado (HEX-4).
pub struct SendWelcomeSubscriber;

impl AccountEventSubscriber for SendWelcomeSubscriber {
    fn on(&self) {}
}
