use shared::Fault;

pub struct Account;

/// El dominio nombra una primitiva de E/S del kernel legacy (HEX-3).
pub fn load() -> Result<Account, Fault> {
    Ok(Account)
}
