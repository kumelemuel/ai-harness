//! Configuración del proceso, leída una vez al arrancar.

use std::fmt;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:3000";

/// Sin `Debug`: lleva un secreto (ERR-4).
pub struct Config {
    pub bind_address: String,
    /// Clave HS256 de los tokens de acceso. Obligatoria: sin ella no hay
    /// manera de verificar a nadie, y un valor por defecto sería un secreto
    /// versionado.
    pub jwt_secret: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing(&'static str),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "falta la variable {name}"),
        }
    }
}

impl Config {
    /// `lookup` es `std::env::var` en producción; los tests pasan un mapa.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let jwt_secret = lookup("JWT_SECRET")
            .filter(|secret| !secret.trim().is_empty())
            .ok_or(ConfigError::Missing("JWT_SECRET"))?;

        Ok(Self {
            bind_address: lookup("BIND_ADDRESS").unwrap_or_else(|| DEFAULT_BIND_ADDRESS.into()),
            jwt_secret: jwt_secret.into_bytes(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        Config::from_lookup(|name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        })
    }

    #[test]
    fn requires_the_jwt_secret() {
        assert_eq!(config(&[]).err(), Some(ConfigError::Missing("JWT_SECRET")));
        assert_eq!(
            config(&[("JWT_SECRET", "  ")]).err(),
            Some(ConfigError::Missing("JWT_SECRET"))
        );
    }

    #[test]
    fn defaults_the_bind_address() {
        let config = config(&[("JWT_SECRET", "s3cret")]).expect("configuración válida");

        assert_eq!(config.bind_address, DEFAULT_BIND_ADDRESS);
        assert_eq!(config.jwt_secret, b"s3cret");
    }
}
