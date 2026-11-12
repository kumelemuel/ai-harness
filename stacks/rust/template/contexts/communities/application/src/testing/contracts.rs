//! Baterías de contrato de los puertos salientes (TDD-2).
//!
//! Una por puerto. Cada caso es una `async fn` que recibe la fábrica del
//! adaptador y no elige ejecutor. La macro de cada batería genera un `#[test]`
//! por caso y recibe el `run` del caller: `block_on` para adaptadores que
//! resuelven de inmediato (fakes, memoria) o un runtime real (Postgres).

pub mod community_repository;
