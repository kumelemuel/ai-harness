use crate::domain;

pub trait RegisterAccountUseCase {
    fn execute(&self) -> domain::Account;
}

pub trait AccountEventSubscriber {
    fn on(&self);
}
