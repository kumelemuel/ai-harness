# Stack — Rust

> **Estado:** v0.2 (2026-11-12)
> **Toolchain:** Rust estable, edición 2024 (verificado con 1.97).
> **Referencia ejecutable:** [`template/`](template/) — compila, todos los tests en verde
> (lo comprueba el [CI del harness](../../.github/workflows/harness.yml)) y los tests de
> arquitectura detectan cada violación (verificado inyectándolas).

Traducción de [`core/principles`](../../core/principles/README.md) a Rust. Los
principios no cambian; aquí se decide **cómo se expresan**.

---

## Decisiones del stack

| ID | Decisión | Motivo |
|----|----------|--------|
| RS-1 | **Un crate por capa, agrupados en carpetas por contexto**, incluido el `api` (driving) de cada contexto | Cargo hace cumplir HEX-1/2/3 al compilar: lo que no está declarado no se puede importar. Cada contexto expone sus rutas; el composition root solo las compone. |
| RS-2 | **Árbol B = `Fault` opaco + `panic!` solo para invariantes** | Rust no tiene excepciones. `Fault` viaja por `?` hasta el borde; `panic!` queda para bugs y lo captura `CatchPanicLayer`. |
| RS-3 | **Puertos async con `BoxFuture` del kernel** | `async fn` en traits no es compatible con `dyn`. `BoxFuture` es solo `std`, permite `Arc<dyn Puerto>` y no rompe HEX-2. |
| RS-4 | **Tests de arquitectura con `toml` + `syn`** en `tools/architecture` | Los tests de dependencias impiden *declarar* lo prohibido; los de nombres cubren lo que el compilador no ve (HEX-4/5/6). |

---

## Disposición (RS-1)

```
Cargo.toml                      workspace: members = ["kernel", "edge/*", "contexts/*/*", "apps/*", "tools/*"]
kernel/                         Result nativo, Fault, Failure, BoxFuture, marcadores. Solo std.
edge/http/                      Borde compartido: Problem Details, AuthenticatedAccount, unexpected(). → kernel
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
│       ├── testing.rs          dobles (Fake…Adapter) + block_on, feature `testing`
│       └── testing/contracts/  {puerto}.rs: batería de contrato + macro {puerto}_contract!
├── infrastructure/             → kernel, {ctx}-domain, {ctx}-application (+ uuid, sqlx…)
└── api/                        → kernel, {ctx}-application, edge-http (+ axum, serde). Nunca domain.
    └── src/                    {Ctx}Api { use cases, faults }.protected()/public() → Router
apps/server/  (o app/)          composition root: construye contextos, compone rutas, autentica
tools/architecture/             tests de arquitectura (VER-1)
```

Nombres de crate: `{ctx}-domain`, `{ctx}-application`, `{ctx}-infrastructure`, `{ctx}-api`.
La carpeta de capa **debe** llamarse `domain`, `application`, `infrastructure` o `api`:
los tests de arquitectura clasifican por ubicación.

**`edge/http` es la única pieza compartida del anillo driving.** Contiene lo
transversal al transporte y solo ve el kernel; nunca tipos de un contexto. Un
`{ctx}-api` puede depender de `edge-http` y de **su propio** `{ctx}-application`,
nada más. (Un `apps/api` central, que ve todos los `application`, sigue siendo
válido si se prefiere un solo crate HTTP.)

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
| Transporte (HEX-6) | `CreateCommunityHttpRequest` (`Deserialize`), `CreateCommunityHttpResponse` (`Serialize`) en `{ctx}-api` |
| Identidad autenticada | El middleware del composition root inserta `edge_http::AuthenticatedAccount` como `Extension`; el handler la lee con `Option<Extension<…>>` y responde 401 si falta |
| Agregado inmutable (HEX-7) | Campos privados, getters, comportamiento `fn rename(&self, …) -> Result<Self, NotCommunityOwner>` |
| Value object | Newtype con `new(String) -> Result<Self, Invalid…>`; implementa `kernel::ValueObject` |
| Composition root (HEX-2) | `apps/server/src/main.rs`: construye adaptadores y handlers, arma cada `{Ctx}Api { … }` y compone sus `Router` |
| Capacidades del kernel (HEX-8) | `kernel::IdGenerator` y `kernel::FaultReporter` (traits; `Clock` es la tercera de la lista, aún sin uso en la plantilla). Sin sufijo de rol. Los adaptadores siguen HEX-5 con el nombre completo: `UuidIdGeneratorAdapter` en infraestructura. `uuid` nunca en el núcleo. |

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
| Error de dominio | Solo `Debug` + `Display`; **no implementa `Error`**. Así `?` no lo convierte en `Fault`. Si un dato persistido ya no cumple la regla, es dato corrupto: se convierte a mano y con contexto, `.map_err(\|e\| Fault::new(format!("slug persistido inválido: {e}")))?` |
| Error de dominio → hoja | `.map_err(\|_\| Failure::expected(Validation { field: "slug" }))?`. Sin envolver: nunca `FromDomain(InvalidCommunitySlug)`. |
| Puerto sin errores declarados | `Result<T, Fault>`; en el handler, `.await?` lo convierte en `Failure::Unexpected` |
| Caso de uso sin árbol A (ERR-1) | `BoxFuture<'_, Result<Response, Fault>>`: sin tipo de error ni `Failure` |
| Puerto con errores declarados (ERR-3) | `Result<T, Failure<{Contrato}Error>>`; el handler traduce con `.map_err(\|f\| f.map_expected(…))` |
| Falla técnica en un adaptador | `?` convierte cualquier `E: Error` en `Fault` (`impl From<E> for Fault`) |
| No encontrado | `find_*` devuelve `Option`; el handler decide si es `NotFound` o `Conflict` |
| Invariante roto / bug | `panic!` o `.expect("invariante: …")`. **`unwrap` está prohibido** (clippy `unwrap_used = "deny"`): no dice qué invariante se rompió. |
| Borde (ERR-4) | `match` exhaustivo: `Expected(e)` → helper por hoja que devuelve `edge_http::Problem` con su `code`; `Unexpected(f)` → `edge_http::unexpected` (reporta + 500 genérico); `panic!` → `CatchPanicLayer` |
| Código estable para clientes | `Problem::conflict(…).with_code("COMMUNITIES_SLUG_ALREADY_EXISTS")`. Los códigos viven en `{ctx}-api/src/codes.rs`: son vocabulario de transporte |

**Por qué `Fault` no implementa `Error`.** Permite el `impl<E: Error> From<E> for Fault`
(mismo truco que `anyhow`), así `?` funciona con `sqlx::Error`, `io::Error`, etc.
dentro de los adaptadores. A la vez, `?` **no** convierte un error de dominio ni en
`Failure<E>` (no existe `From<ErrorDeDominio> for Failure<E>`) ni en `Fault`. Esto
último es así porque los errores de dominio no implementan `Error`, así que el
`impl From<E: Error>` no los alcanza. En los dos casos el compilador obliga a escribir
la traducción. Si alguien agrega `impl Error` a un error de dominio, el `?` hacia
`Fault` vuelve a compilar y el fallo de negocio termina como un 500 genérico. Por eso
un test de arquitectura lo impide (ERR-1). El compilador incluso sugiere ese `impl`
como arreglo del E0277.

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
| Caso de uso | `CreateCommunityHandler::new(Arc::new(FakeCommunityAdapter::empty()), …)` + `block_on` | `handler.rs` |
| Contrato del puerto | `community_repository_contract! { new_adapter: …, run: … }` contra el doble y contra cada adaptador | `application/src/testing.rs` (doble) y el archivo de cada adaptador |
| Adaptador | La batería de contrato, más lo propio de su tecnología (Postgres con testcontainers o similar) | crate de infraestructura |
| Transporte | `{Ctx}Api { … }.protected().oneshot(request)` con un `UseCase` stub; la identidad se inyecta con `request.extension(AuthenticatedAccount { … })` | `contexts/{ctx}/api/tests/` |
| Arquitectura | `cargo test -p architecture` | `tools/architecture/tests/` |

- **Dobles** en `application/src/testing.rs`, compilados con `cfg(any(test, feature = "testing"))`.
  Otras capas los usan con `[dev-dependencies] {ctx}-application = { workspace = true, features = ["testing"] }`.
  Viven en `application` porque sus tests no ven `infrastructure` (HEX-1). Se nombran como
  adaptadores: `FakeCommunityAdapter` (con estado), `FixedIdGeneratorAdapter` (respuesta fija).
  `hex_5_adapters_*` también recorre `testing`.
- **`InMemoryCommunityAdapter`** es un adaptador de producción (correr sin base de datos), no un
  doble. Pasa la misma batería que el fake.
- **`block_on`** del módulo `testing`: ejecutor mínimo con `std` (`Waker::noop`). Solo para
  futures que resuelven de inmediato (fakes); el núcleo no necesita `tokio` ni siquiera en
  tests, y `hex_2_core_*` lo impide (TDD-2).
- Los tests de caso de uso afirman con `matches!(result, Err(Failure::Expected(…)))`.

### Baterías de contrato

Cada puerto saliente tiene una en `application/src/testing/contracts/{puerto}.rs`. Cada caso
es una `pub async fn {caso}<R: Puerto>(new_adapter: impl Fn() -> R)`. La macro
`{puerto}_contract!` (exportada en la raíz del crate) genera un `#[test]` por caso, dentro de
un módulo `{puerto}_contract`:

```rust
// application/src/testing.rs: el doble
crate::community_repository_contract! {
    new_adapter: FakeCommunityAdapter::empty(),
    run: block_on,
}

// infrastructure/src/in_memory_community.rs: el adaptador en memoria
communities_application::community_repository_contract! {
    new_adapter: InMemoryCommunityAdapter::default(),
    run: block_on,
}

// infrastructure/src/postgres_community.rs: un adaptador con IO real
communities_application::community_repository_contract! {
    new_adapter: PostgresCommunityAdapter::new(pool()),
    run: |future| tokio::runtime::Runtime::new().expect("invariante: runtime").block_on(future),
}
```

- **Por qué `async` sin ejecutor.** `block_on` solo sirve para futures que resuelven de
  inmediato, y un adaptador con IO necesita un runtime real. La batería no elige: cada caller
  pasa su `run`. `application` sigue sin `tokio` (HEX-2). `infrastructure` puede declararlo
  como dev-dependency.
- **Por qué una macro.** Genera un test por caso, así un fallo nombra el caso y el resto sigue
  corriendo. Además, un caso nuevo corre contra todas las implementaciones sin tocarlas.
- **Por qué una fábrica.** Cada caso empieza con un adaptador vacío. Si el puerto declara
  errores (ERR-3), la fábrica recibe el escenario que los provoca
  (`new_adapter: impl Fn(Scenario) -> R`, con una variante de `Scenario` por error declarado), y
  la batería tiene un caso por variante.

**Puerto nuevo:**

1. Escribe `testing/contracts/{puerto}.rs`, con un caso por comportamiento del que dependan
   los handlers y uno por variante de error declarada. Agrega su macro y el `pub mod`.
2. Invoca la macro en `testing.rs`, contra el doble, y ve los casos en rojo.
3. Implementa el doble hasta que pasen. Después, cada adaptador invoca la macro en su archivo.
- En crates de test de integración, `#![allow(clippy::unwrap_used)]` al inicio del archivo.

---

## Enforcement (RS-4, VER-1)

| Regla | Mecanismo | Falla en |
|-------|-----------|----------|
| HEX-1 dependencias hacia adentro, `api` solo ve el `application` de su contexto y `edge`, sin cruces entre contextos | `tests/dependencies.rs::hex_1_*` (lee los `Cargo.toml`) + Cargo | `cargo test` |
| HEX-2 núcleo solo con `std` + kernel; TDD-2 sus tests tampoco usan runtime ni framework (lee también `[dev-dependencies]`) | `tests/dependencies.rs::hex_2_core_*` | `cargo test` |
| HEX-2 frameworks de servidor solo en `api`, `edge` y bootstrap | `tests/dependencies.rs::hex_2_server_*` | `cargo test` |
| HEX-3 `domain` no ve `application` | Cargo (no está declarado) + `hex_1` | compilación |
| HEX-4 `UseCase` de un solo `execute`; implementaciones `*Handler`; `api` nunca nombra un `*Handler` | `tests/naming.rs::hex_4_*` (`syn`) | `cargo test` |
| HEX-4 el contrato no expone `domain`: campos públicos y variantes de `*Command`/`*Query`/`*Response`/`*Error`, firmas de `ports/inbound` y `pub use` del dominio en `application`, con ruta completa o con nombre importado | `tests/naming.rs::hex_4_use_case_contracts_do_not_expose_domain_types` (`syn`). Cargo no basta: `api` puede usar un valor de dominio que le llega por un campo público sin declarar el crate | `cargo test` |
| HEX-5 puertos salientes con sufijo de rol, nunca `Port` | `tests/naming.rs::hex_5_outbound_ports_are_named_by_role` | `cargo test` |
| HEX-5 adaptadores `{Prefijo}{Concepto}Adapter`: en `infrastructure` y en `application/src/testing*` (dobles), todo tipo que implementa un puerto de `ports/outbound` de su contexto o una capacidad del kernel. Concepto es el puerto sin su rol, o la capacidad completa | `tests/naming.rs::hex_5_adapters_are_named_by_technology_and_concept` (`syn`) | `cargo test` |
| HEX-8 el kernel no declara dependencias (ni de test ni por `target`), y sus traits públicos son marcadores de modelo o capacidades de la lista cerrada | `tests/kernel.rs::hex_8_*` (`toml` + `syn`) | `cargo test` |
| HEX-6 vocabulario de DTOs y `{Caso}Http*` en `api` | `tests/naming.rs::hex_6_*` | `cargo test` |
| ERR-1/RS-2 ningún error de dominio implementa `std::error::Error` (tampoco dentro de un `macro_rules!`) | `tests/errors.rs::err_1_*` (`syn`) + compilador: `?` no convierte el error en `Fault` | `cargo test` |
| `unwrap` prohibido; `unsafe` prohibido | `[workspace.lints]` | `cargo clippy -D warnings` |
| HEX-1…6, HEX-8, ERR-1 mientras el agente edita | Hook `PostToolUse` de craft-backend: `rustfmt` + `cargo test -p architecture` | cada edición de `.rs`/`Cargo.toml` (~1 s) |
| Todo lo anterior antes de cerrar | Hook `Stop` de craft-backend: `clippy --no-deps -D warnings` sobre los crates de la arquitectura + `cargo test --workspace` | al terminar, si hay cambios Rust sin commit |
| TDD-2 / ERR-3 el doble y los adaptadores cumplen el mismo contrato del puerto | Batería de contrato (`application::testing::contracts`) | `cargo test` |
| HEX-7, ERR-1…3, TDD-1 (test antes o junto a la producción en el diff y en el orden de commits) | `architecture-reviewer`. Los hooks garantizan verde, no rojo primero | revisión |

**Hooks del plugin.** Se activan solos en cualquier workspace con
`tools/architecture/Cargo.toml`; fuera de eso no hacen nada. Se pueden apagar con
`CRAFT_BACKEND_HOOKS=0` (ambos) o `CRAFT_BACKEND_STOP_GATE=0` (solo Stop). El gate de
Stop bloquea una sola vez por turno. Si el fallo lo causó el cambio del agente, lo
corrige antes de terminar (VER-1). Si el rojo es intencional (fase roja de TDD) o el
agente cree que el test está mal, no lo modifica: explica el estado y por qué, y
termina (TDD-1).

**CI mínimo:**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Migración incremental

Un workspace puede tener contextos ya separados por capas y otros *legacy* (un
solo crate en `contexts/{ctx}/Cargo.toml`) mientras se migra:

- Los tests de arquitectura **ignoran** los contextos legacy y los crates fuera de
  `kernel/`, `edge/`, `contexts/{ctx}/{capa}/`, `apps/` y `app/` (p. ej. un `shared/`).
- Un crate migrado que dependa de uno legacy desde `domain`/`application` falla en
  HEX-2 (lo ve como crate externo): el núcleo migrado no puede apoyarse en legacy.
- El gate de Stop aplica clippy solo a los crates de la arquitectura (`--no-deps -p …`);
  los tests sí corren en todo el workspace. Así el código legacy con warnings no
  bloquea, pero un test legacy roto sí.
- Con contextos legacy, lista los miembros explícitamente
  (`"contexts/iam", "contexts/communities/*"`): el glob `contexts/*/*` tomaría
  `contexts/iam/src` como miembro y `exclude` no lo evita.

## Violaciones típicas en Rust

| En vez de | Usar |
|-----------|------|
| Un crate por contexto con capas como módulos | Un crate por capa (RS-1) |
| Rutas HTTP dentro de `infrastructure` junto a la persistencia | Crate `{ctx}-api` separado; `api` e `infrastructure` no se ven (HEX-1) |
| `uuid`, `serde`, `chrono` en `domain`/`application` | Solo `std` + kernel; la implementación va en infraestructura |
| `trait CommunityRepositoryPort` | `trait CommunityRepository` |
| `PostgresCommunityRepositoryAdapter`, `UuidIdGenerator` | `PostgresCommunityAdapter`, `UuidIdGeneratorAdapter` (HEX-5) |
| Un trait de negocio en el kernel porque dos contextos lo usan (`CommunityLookup`) | Un puerto en el `application` de cada contexto, o un `Client`. El kernel solo admite las capacidades de HEX-8 |
| Inbound `CommunityCreationPort` + impl `CreateCommunityUseCase` | Inbound `CreateCommunityUseCase` + impl `CreateCommunityHandler` |
| `impl Error for InvalidCommunitySlug` (o en `domain_error!`) "para que `?` compile" | Solo `Debug` + `Display`; traducir a una hoja, o a `Fault` a mano si es dato corrupto (ERR-1) |
| Un `SystemError` global en todas las firmas | `Failure<{Verb}{Noun}Error>` acotado |
| Errores clasificados por capa (`Domain`/`Application`/`Infrastructure`) | Clasificados por árbol: `Expected` / `Unexpected` |
| Mapear a HTTP por código de error string (`match code { "X" => 409 }`) | `match` exhaustivo sobre el enum del contrato |
| `async-trait` en el núcleo | `BoxFuture` del kernel |
| Importar el dominio de otro contexto | Vista propia del concepto o puerto `Client` |
| `.unwrap()` | `.expect("invariante: …")` o propagar con `?` |
| `.env` con secretos versionado | `.env` en `.gitignore`; `.env.example` sin valores |
