# Stack — Rust

> **Estado:** v0.1 (2026-11-12)
> **Toolchain:** Rust estable, edición 2024 (verificado con 1.97).
> **Referencia ejecutable:** [`template/`](template/) — compila, 33 tests en verde y
> los tests de arquitectura detectan cada violación (verificado inyectándolas).

Traducción de [`core/principles`](../../core/principles/README.md) a Rust. Los
principios no cambian; aquí se decide **cómo se expresan**.

---

## Decisiones del stack

| ID | Decisión | Motivo |
|----|----------|--------|
| RS-1 | **Un crate por capa, agrupados en carpetas por contexto** | Cargo hace cumplir HEX-1/2/3 al compilar: lo que no está declarado no se puede importar. La carpeta conserva la cohesión del contexto. |
| RS-2 | **Árbol B = `Fault` opaco + `panic!` solo para invariantes** | Rust no tiene excepciones. `Fault` viaja por `?` hasta el borde; `panic!` queda para bugs y lo captura `CatchPanicLayer`. |
| RS-3 | **Puertos async con `BoxFuture` del kernel** | `async fn` en traits no es compatible con `dyn`. `BoxFuture` es solo `std`, permite `Arc<dyn Puerto>` y no rompe HEX-2. |
| RS-4 | **Tests de arquitectura con `toml` + `syn`** en `tools/architecture` | Los tests de dependencias impiden *declarar* lo prohibido; los de nombres cubren lo que el compilador no ve (HEX-4/5/6). |

---

## Disposición (RS-1)

```
Cargo.toml                      workspace: members = ["kernel", "contexts/*/*", "apps/*", "tools/*"]
kernel/                         Result nativo, Fault, Failure, BoxFuture, marcadores. Solo std.
contexts/{ctx}/
├── domain/                     → kernel
├── application/                → kernel, {ctx}-domain
│   └── src/
│       ├── ports/inbound/      {verb}_{noun}.rs  → trait {Verb}{Noun}UseCase
│       ├── ports/outbound/     {concepto}_{rol}.rs → trait {Concepto}{Rol}
│       │   └── dtos/           {Contrato}Payloads (si hace falta)
│       ├── commands/{caso}/    command.rs · response.rs · error.rs · handler.rs
│       ├── queries/{caso}/     query.rs · response.rs · error.rs · handler.rs
│       ├── errors.rs           hojas compartidas (ERR-2)
│       └── testing.rs          fakes + block_on, feature `testing`
└── infrastructure/             → kernel, {ctx}-domain, {ctx}-application (+ uuid, sqlx…)
apps/
├── api/                        → kernel, *-application (+ axum, serde). Nunca domain.
└── server/                     composition root: ve todo, nadie depende de él
tools/architecture/             tests de arquitectura (VER-1)
```

Nombres de crate: `{ctx}-domain`, `{ctx}-application`, `{ctx}-infrastructure`.
La carpeta de capa **debe** llamarse `domain`, `application` o `infrastructure`:
los tests de arquitectura clasifican por ubicación.

**Herencia del workspace.** Cada `Cargo.toml` de capa queda en ~10 líneas:

```toml
[package]
name = "communities-domain"
version.workspace = true
edition.workspace = true
publish.workspace = true

[dependencies]
kernel.workspace = true

[lints]
workspace = true
```

Las versiones de crates externos se declaran una sola vez en
`[workspace.dependencies]`.

**Sin dependencias entre contextos.** Un contexto no importa el dominio de otro
(p. ej. `AccountId` de `iam` dentro de `communities`): modela su propia vista
(`OwnerId`). Si necesita datos de otro contexto, lo pide por un puerto `Client`.

---

## Traducción de conceptos

| Principio | Rust |
|-----------|------|
| Puerto entrante (HEX-4) | `pub trait CreateCommunityUseCase: Send + Sync { fn execute(&self, command) -> BoxFuture<'_, Result<Response, Failure<Error>>>; }` |
| Handler (HEX-4) | `pub struct CreateCommunityHandler { communities: Arc<dyn CommunityRepository>, … }` con `new(…)`; implementa el `UseCase` |
| DTOs del caso de uso | `CreateCommunityCommand`, `CreateCommunityResponse`, `CreateCommunityError`, cada uno en su archivo dentro de `commands/create_community/`, reexportados en `mod.rs` |
| Contrato sin tipos de dominio (HEX-4) | `Response` con `String`/primitivos; `impl From<&Community> for CreateCommunityResponse` |
| Puerto saliente (HEX-5) | `pub trait CommunityRepository: Send + Sync` con métodos que devuelven `BoxFuture<'a, Result<T, Fault>>` |
| Adaptador (HEX-5) | `InMemoryCommunityAdapter`, `PostgresCommunityAdapter` |
| Transporte (HEX-6) | `CreateCommunityHttpRequest` (`Deserialize`), `CreateCommunityHttpResponse` (`Serialize`) en `apps/api` |
| Agregado inmutable (HEX-7) | Campos privados, getters, comportamiento `fn rename(&self, …) -> Result<Self, NotCommunityOwner>` |
| Value object | Newtype con `new(String) -> Result<Self, Invalid…>`; implementa `kernel::ValueObject` |
| Composition root (HEX-2) | `apps/server/src/main.rs`: construye adaptadores y handlers; `AppState` guarda `Arc<dyn …UseCase>` |
| Generación de IDs | `kernel::IdGenerator` (trait); la implementación `UuidIdGenerator` vive en infraestructura. `uuid` nunca en el núcleo. |

---

## Errores (RS-2)

```rust
// kernel
pub struct Fault { /* opaco */ }        // árbol B; no implementa Error a propósito
pub enum Failure<E> {
    Expected(E),        // árbol A
    Unexpected(Fault),  // árbol B
}
```

| Situación | Cómo se expresa |
|-----------|-----------------|
| Hoja compartida (ERR-2) | `pub struct Conflict { pub resource: &'static str }` en `application/src/errors.rs` |
| Contrato acotado (ERR-2) | `pub enum CreateCommunityError { Validation(Validation), Conflict(Conflict) }` + `From<Hoja>` por variante. Las variantes del enum **son** la declaración de qué hojas admite. |
| Error de dominio → hoja | `.map_err(\|_\| Failure::expected(Validation { field: "slug" }))?`. Sin envolver: nunca `FromDomain(InvalidCommunitySlug)`. |
| Puerto sin errores declarados | `Result<T, Fault>`; en el handler, `.await?` lo convierte en `Failure::Unexpected` |
| Puerto con errores declarados (ERR-3) | `Result<T, Failure<{Contrato}Error>>`; el handler traduce con `.map_err(\|f\| f.map_expected(…))` |
| Falla técnica en un adaptador | `?` convierte cualquier `E: Error` en `Fault` (`impl From<E> for Fault`) |
| No encontrado | `find_*` devuelve `Option`; el handler decide si es `NotFound` o `Conflict` |
| Invariante roto / bug | `panic!` o `.expect("invariante: …")`. **`unwrap` está prohibido** (clippy `unwrap_used = "deny"`): no dice qué invariante se rompió. |
| Borde (ERR-4) | `match` exhaustivo: `Expected(e)` → helper Problem Details por hoja; `Unexpected(f)` → `FaultReporter::report` + 500 genérico; `panic!` → `CatchPanicLayer` |

**Por qué `Fault` no implementa `Error`.** Permite el `impl<E: Error> From<E> for Fault`
(mismo truco que `anyhow`), así `?` funciona con `sqlx::Error`, `io::Error`, etc.
dentro de los adaptadores. A la vez, `?` **no** convierte un error de dominio en
`Failure<E>`: el compilador obliga a traducirlo explícitamente a una hoja.

---

## Async (RS-3)

```rust
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// Implementación async real
fn save<'a>(&'a self, c: &'a Community) -> BoxFuture<'a, Result<(), Fault>> {
    Box::pin(async move { sqlx::query(…).execute(&self.pool).await?; Ok(()) })
}

// Implementación síncrona (memoria, fakes): sin `async`
fn save<'a>(&'a self, c: &'a Community) -> BoxFuture<'a, Result<(), Fault>> {
    let result = /* … */;
    Box::pin(std::future::ready(result))
}
```

Costo: una asignación por llamada. Si un día se vuelve relevante, se puede pasar a
genéricos en los caminos calientes sin cambiar los principios.

---

## Tests por capa (TDD-2)

| Capa | Cómo | Dónde |
|------|------|-------|
| Kernel / dominio | `#[test]` puro | `#[cfg(test)] mod tests` en el mismo archivo |
| Caso de uso | `CreateCommunityHandler::new(Arc::new(FakeCommunityRepository::empty()), …)` + `block_on` | `handler.rs` |
| Adaptador | Contra su tecnología (en memoria directo; Postgres con testcontainers o similar) | crate de infraestructura |
| Transporte | `api::router(state).oneshot(request)` con un `UseCase` stub | `apps/api/tests/` |
| Arquitectura | `cargo test -p architecture` | `tools/architecture/tests/` |

- **Fakes** en `application/src/testing.rs`, compilados con `cfg(any(test, feature = "testing"))`.
  Otras capas los usan con `[dev-dependencies] {ctx}-application = { workspace = true, features = ["testing"] }`.
- **`block_on`** del módulo `testing`: ejecutor mínimo con `std` (`Waker::noop`). Solo para
  futures que resuelven de inmediato (fakes); el núcleo no necesita `tokio` ni siquiera en
  tests.
- Los tests de caso de uso afirman con `matches!(result, Err(Failure::Expected(…)))`.
- En crates de test de integración, `#![allow(clippy::unwrap_used)]` al inicio del archivo.

---

## Enforcement (RS-4, VER-1)

| Regla | Mecanismo | Falla en |
|-------|-----------|----------|
| HEX-1 dependencias hacia adentro, `api` solo ve `application`, sin cruces entre contextos | `tests/dependencies.rs::hex_1_*` (lee los `Cargo.toml`) + Cargo | `cargo test` |
| HEX-2 núcleo solo con `std` + kernel | `tests/dependencies.rs::hex_2_core_*` | `cargo test` |
| HEX-2 frameworks de servidor solo en `api`/`server` | `tests/dependencies.rs::hex_2_server_*` | `cargo test` |
| HEX-3 `domain` no ve `application` | Cargo (no está declarado) + `hex_1` | compilación |
| HEX-4 `UseCase` de un solo `execute`; implementaciones `*Handler`; `api` nunca nombra un `*Handler` | `tests/naming.rs::hex_4_*` (`syn`) | `cargo test` |
| HEX-5 sufijo de rol, nunca `Port` | `tests/naming.rs::hex_5_*` | `cargo test` |
| HEX-6 vocabulario de DTOs y `{Caso}Http*` en `api` | `tests/naming.rs::hex_6_*` | `cargo test` |
| `unwrap` prohibido; `unsafe` prohibido | `[workspace.lints]` | `cargo clippy -D warnings` |
| HEX-7, ERR-1…3 | `architecture-reviewer` | revisión |

**CI mínimo:**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Violaciones típicas en Rust

| En vez de | Usar |
|-----------|------|
| Un crate por contexto con capas como módulos | Un crate por capa (RS-1) |
| `uuid`, `serde`, `chrono` en `domain`/`application` | Solo `std` + kernel; la implementación va en infraestructura |
| `trait CommunityRepositoryPort` | `trait CommunityRepository` |
| Inbound `CommunityCreationPort` + impl `CreateCommunityUseCase` | Inbound `CreateCommunityUseCase` + impl `CreateCommunityHandler` |
| Un `SystemError` global en todas las firmas | `Failure<{Verb}{Noun}Error>` acotado |
| Errores clasificados por capa (`Domain`/`Application`/`Infrastructure`) | Clasificados por árbol: `Expected` / `Unexpected` |
| Mapear a HTTP por código de error string (`match code { "X" => 409 }`) | `match` exhaustivo sobre el enum del contrato |
| `async-trait` en el núcleo | `BoxFuture` del kernel |
| Importar el dominio de otro contexto | Vista propia del concepto o puerto `Client` |
| `.unwrap()` | `.expect("invariante: …")` o propagar con `?` |
| `.env` con secretos versionado | `.env` en `.gitignore`; `.env.example` sin valores |
