/// Puerto saliente con rol `Service`: es el uso permitido del sufijo (HEX-7).
pub trait PasswordHashingService {
    fn hash(&self, plain: &str) -> String;
}
