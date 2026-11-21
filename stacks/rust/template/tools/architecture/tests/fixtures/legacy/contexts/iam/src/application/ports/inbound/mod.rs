use crate::domain;

pub trait RegisterAccountUseCase {
    fn execute(&self) -> domain::Account;
}
