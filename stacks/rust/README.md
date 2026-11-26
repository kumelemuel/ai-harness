# Stack — Rust

> **Estado:** v0.2 (2026-11-12)
> **Toolchain:** Rust estable, edición 2024 (verificado con 1.97).
> **Referencia ejecutable:** [`template/`](template/) — compila, todos los tests en verde
> (lo comprueba el [CI del harness](../../.github/workflows/harness.yml), que además corre
> clippy con el último nightly cada semana, para que la copia de `tools/architecture` pase
> también en un proyecto que fija un nightly) y los tests de arquitectura detectan cada violación (verificado inyectándolas).

Traducción de [`core/principles`](../../core/principles/README.md) a Rust. Los
principios no cambian; aquí se decide **cómo se expresan**.

---

## Decisiones del stack

| ID | Decisión | Motivo |
|----|----------|--------|
| RS-1 | **Un crate por capa, agrupados en carpetas por contexto**, incluido el `api` (driving) de cada contexto. Las versiones viven en `[workspace.dependencies]` y cada miembro las hereda | Cargo hace cumplir HEX-1/2/3 al compilar: lo que no está declarado no se puede importar. Cada contexto expone sus rutas; el composition root solo las compone. Una versión por dependencia evita que dos capas diverjan. |
| RS-2 | **Árbol B = `Fault` opaco + `panic!` solo para invariantes** | Rust no tiene excepciones. `Fault` viaja por `?` hasta el borde; `panic!` queda para bugs y lo captura `CatchPanicLayer`. |
| RS-3 | **Puertos async con `BoxFuture` del kernel** | `async fn` en traits no es compatible con `dyn`. `BoxFuture` es solo `std`, permite `Arc<dyn Puerto>` y no rompe HEX-2. |
| RS-4 | **Tests de arquitectura con `toml` + `syn`** en `tools/architecture` | Los tests de dependencias impiden *declarar* lo prohibido; los de nombres cubren lo que el compilador no ve (HEX-4/5/6). |

---

## Disposición (RS-1)

```
Cargo.toml                      workspace: members = ["kernel", "edge/*", "infrastructure/*", "contexts/*/*", "apps/*", "tools/*"]
kernel/                         Result nativo, Fault, Failure, BoxFuture, marcadores y capacidades
                                (Clock, IdGenerator, FaultReporter). Solo std.
edge/http/                      Borde compartido: Problem Details, AuthenticatedAccount, unexpected(). → kernel
infrastructure/{tecnología}/    Infraestructura compartida: adaptadores de las capacidades del kernel
                                (infrastructure/stderr: StderrFaultReporterAdapter;
                                infrastructure/system: SystemClockAdapter;
                                infrastructure/uuid: UuidIdGeneratorAdapter). → kernel
                                Nunca en un contexto (hex_8_capability_adapters_*).
contexts/{ctx}/
├── domain/                     → kernel
├── application/                → kernel, {ctx}-domain
│   └── src/
│       ├── ports/inbound/      {verb}_{noun}.rs  → trait {Verb}{Noun}UseCase
│       ├── ports/outbound/     {concepto}_{rol}.rs → trait {Concepto}{Rol}
│       │   └── dtos/           {concepto}_payloads.rs: el contenedor {Contrato}Payloads es el módulo (si hace falta)
│       ├── commands/{caso}/    command.rs · response.rs · error.rs · handler.rs
│       ├── queries/{caso}/     query.rs · response.rs · error.rs · handler.rs
│       ├── subscribers/        {verb_noun}.rs → {Verb}{Noun}Subscriber (impl {Concepto}EventSubscriber)
│       ├── workflows/{verb_noun}/  workflow.rs ({Verb}{Noun}Workflow) · error.rs ({Verb}{Noun}WorkflowError)
│       ├── errors.rs           hojas compartidas (ERR-2)
│       ├── testing.rs          dobles (Fake…Adapter) + block_on, feature `testing`
│       └── testing/contracts/  {puerto}.rs: batería de contrato + macro {puerto}_contract!
├── infrastructure/             → kernel, {ctx}-domain, {ctx}-application (+ sqlx…)
└── api/                        → kernel, {ctx}-application, edge-http (+ axum, serde). Nunca domain.
    └── src/                    {Ctx}Api { use cases, faults }.protected()/public() → Router
apps/server/  (o app/)          composition root: cablea, no implementa
└── src/
    ├── main.rs                 lee la configuración, compone las rutas: iam.protect(ctx.protected())
    ├── config.rs               variables de entorno, una vez al arrancar (JWT_SECRET obligatorio)
    └── contexts/{ctx}.rs       construye adaptadores y handlers del contexto, devuelve su {Ctx}Api
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

**`infrastructure/*` es la pieza compartida del anillo driven.** Los adaptadores de
producción viven en infraestructura, también los de las capacidades del kernel (HEX-8). Uno
que no pertenece a ningún contexto, como `StderrFaultReporterAdapter`, va en un crate
`infrastructure-{tecnología}` que solo ve el kernel. No va en el composition root, que cablea
pero no implementa, ni en el `infrastructure` de un contexto, que lo ataría a ese contexto. Tampoco
en `edge/http`: registrar una falla no es transporte, y un borde gRPC usaría el mismo
adaptador. Los tests de arquitectura lo clasifican como `Infrastructure` sin contexto, así que
HEX-1 no le deja ver el `application` de ningún contexto.

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

Las versiones de los crates se declaran una sola vez en
`[workspace.dependencies]`, y cada miembro las hereda con `dep.workspace = true`
(puede agregar `features`). Lo exige `rs_1_dependencies_inherit_from_the_workspace`
en cada miembro de `[workspace] members` (contextos legacy y raíz con `[package]`
incluidos), también en `[dev-dependencies]`, `[build-dependencies]` y `[target.'cfg(…)'.*]`.
Los tests de dependencias (HEX-1, HEX-2) resuelven los renombres
(`foo = { package = "axum" }`, también en la raíz): cuenta el paquete, no la clave.

**Sin dependencias entre contextos.** Un contexto no importa el dominio de otro
(p. ej. `AccountId` de `iam` dentro de `communities`): modela su propia vista
(`OwnerId`). Si necesita datos de otro contexto, lo pide por un puerto `Client`.

---

## Traducción de conceptos

| Principio | Rust |
|-----------|------|
| Puerto entrante (HEX-4) | `pub trait CreateCommunityUseCase: Send + Sync { fn execute(&self, command) -> BoxFuture<'_, Result<Response, Failure<Error>>>; }` |
| Handler (HEX-4) | `pub struct CreateCommunityHandler { communities: Arc<dyn CommunityRepository>, … }` con `new(…)`; implementa el `UseCase` |
| Workflow (HEX-4) | `pub struct FollowUpWorkflow { … Arc<dyn Puerto> … }` con `pub async fn run(&self, …) -> Result<T, Failure<FollowUpWorkflowError>>`: un tipo concreto, sin trait, que los handlers reciben como `Arc<FollowUpWorkflow>`. Cada handler hace `.map_err(\|f\| f.map_expected(…))` hacia su contrato. **Aún sin ejemplo en la plantilla:** `communities` tiene un solo caso de uso |
| DTOs del caso de uso | `CreateCommunityCommand`, `CreateCommunityResponse`, `CreateCommunityError`, cada uno en su archivo dentro de `commands/create_community/`, reexportados en `mod.rs` |
| Contrato sin tipos de dominio (HEX-4) | `Response` con `String`/primitivos; `impl From<&Community> for CreateCommunityResponse` |
| Puerto saliente (HEX-5) | `pub trait CommunityRepository: Send + Sync` con métodos que devuelven `BoxFuture<'a, Result<T, Fault>>` |
| Adaptador (HEX-5) | `InMemoryCommunityAdapter`, `PostgresCommunityAdapter`, `NoOpModerationAdapter` |
| Frontera transaccional (HEX-5) | Puerto `UnitOfWork` en `ports/outbound`. **Aún sin ejemplo en la plantilla:** con un único adaptador en memoria no hay transacción que delimitar, y en Rust la firma natural (`fn execute<T>(…)`) es genérica, así que no admite `Arc<dyn UnitOfWork>`. Se diseña con el primer adaptador con transacción real (p. ej. `PostgresUnitOfWorkAdapter` sobre `sqlx::Transaction`) |
| DTOs de puerto saliente (HEX-6) | `ports/outbound/dtos/moderation_payloads.rs`: `ReviewContentRequest` (entrada), `ModerationVerdict` (salida, `Approved` o `Rejected { reason }`) y `ModerationError` (error declarado). Rust no tiene un tipo contenedor: el módulo `{concepto}_payloads` hace de `{Contrato}Payloads`, y `dtos/mod.rs` reexporta sus tipos |
| Transporte (HEX-6) | `CreateCommunityHttpRequest` (`Deserialize`), `CreateCommunityHttpResponse` (`Serialize`) en `{ctx}-api` |
| Identidad autenticada (HEX-1) | El contexto `iam` declara `VerifyAccessTokenUseCase` (en `queries/`) y el puerto `AccessTokenService`, que implementa `JwtAccessTokenAdapter`. Un token inválido es `AccessTokenVerdict::Invalid`, un veredicto y no un error (ERR-3). `IamApi::protect(routes)`, en `iam-api`, es el middleware: lee `Authorization: Bearer …`, ejecuta el caso de uso, inserta `edge_http::AuthenticatedAccount` como `Extension` y, si no hay identidad, responde 401 sin llegar a la ruta. El handler de otro contexto la lee con `Option<Extension<…>>` y responde 401 si falta, por si la ruta quedó sin proteger |
| Agregado inmutable (HEX-7) | Campos privados, getters, comportamiento `fn rename(&self, …) -> Result<Self, NotCommunityOwner>` |
| Eventos de dominio (HEX-7) | `CommunityEvent` (`enum`, `impl kernel::DomainEvent`) en `domain/src/events.rs`. `Community` guarda `events: Vec<CommunityEvent>`: `create` registra `Created`, y `rename` devuelve el estado nuevo con `Renamed` agregado. `take_events(self) -> (Self, Vec<CommunityEvent>)` los separa. El handler guarda el agregado sin eventos y publica con `CommunityEventPublisher::publish(&events)`. Adaptador `InProcessCommunityEventAdapter::new(vec![…])`, en `communities-infrastructure`: cada evento, en orden, a cada suscriptor, y la primera falla corta. Suscriptor de ejemplo: `RecordCommunityHistorySubscriber`, sobre `CommunityHistoryRepository` (`InMemoryCommunityHistoryAdapter`). Contrato del suscriptor: `trait CommunityEventSubscriber { fn on<'a>(&'a self, event: &'a CommunityEvent) -> BoxFuture<'a, Result<(), Fault>>; }`. Doble del publicador: `SpyCommunityEventAdapter`. La batería del repositorio guarda agregados sin eventos pendientes, la forma persistida |
| Crear o reconstituir (HEX-7) | Dos funciones asociadas del agregado: en Rust, los campos privados ya reservan la construcción, así que no hace falta un `…Factory`. `create(…)` aplica lo que solo vale al nacer: el estado inicial, las reglas de alta y el evento inicial (`Account::create` deja la cuenta pendiente de verificación). `restore(…)` rearma el agregado con los value objects ya leídos de la persistencia, en cualquier estado, sin reglas de alta ni eventos. Si un dato persistido ya no pasa su value object, el adaptador lo convierte en `Fault` (dato corrupto, ver Errores). El id llega de `IdGenerator`, nunca de `uuid` en el dominio |
| Value object | Newtype con `new(String) -> Result<Self, Invalid…>`; implementa `kernel::ValueObject` |
| Servicio de dominio (HEX-7) | Tipo sin campos con funciones asociadas: `pub struct PasswordPolicy;` con `pub fn validate(password: &str) -> Result<(), WeakPassword>`. Sin `Arc<dyn …>` ni `BoxFuture`: los datos los carga el handler |
| Composition root (HEX-2) | `apps/server/src/contexts/{ctx}.rs`: construye adaptadores y handlers y devuelve el `{Ctx}Api { … }` armado. `main.rs` solo compone los `Router`. `config.rs` lee el entorno una vez; un secreto obligatorio que falta detiene el arranque, nunca tiene un valor por defecto, y `Config` no deriva `Debug` |
| Capacidades del kernel (HEX-8) | `kernel::Clock`, `kernel::IdGenerator` y `kernel::FaultReporter` (traits). Sin sufijo de rol. Los adaptadores siguen HEX-5 con el nombre completo, y viven en la infraestructura compartida, nunca en un contexto (`hex_8_capability_adapters_are_shared_infrastructure`): `UuidIdGeneratorAdapter` en `infrastructure-uuid`, `StderrFaultReporterAdapter` en `infrastructure-stderr` (registra toda la cadena de `source()`), `SystemClockAdapter` en `infrastructure-system`. `Clock` todavía no lo usa ningún contexto de la plantilla: el composition root lo cablea con el primero que lo necesite. `uuid` y `chrono` nunca en el núcleo. |

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
| Error de dominio → hoja | `.map_err(\|_\| Failure::expected(Validation { field: Field::Slug }))?`. Sin envolver: nunca `FromDomain(InvalidCommunitySlug)`. `Field` es un enum cerrado del contexto en `errors.rs`; un campo con varios motivos lleva el suyo (`Field::Owner(OwnerRejection)`) |
| Puerto sin errores declarados | `Result<T, Fault>`; en el handler, `.await?` lo convierte en `Failure::Unexpected` |
| Firma que pasa a ser falible (TDD-1) | En el commit de la firma, `.expect("pendiente: propagar el Fault con su test")`: falla fuerte, como el adaptador que antes no podía reportarlo. Nunca `let _ =` ni `.unwrap_or_default()`. El `.await?` llega en el commit siguiente, con `fails_with_unexpected_when_<operación>_fails` |
| Caso de uso sin árbol A (ERR-1) | `BoxFuture<'_, Result<Response, Fault>>`: sin tipo de error ni `Failure` |
| Puerto con errores declarados (ERR-3) | `Result<T, Failure<{Contrato}Error>>`; el handler traduce con `.map_err(\|f\| f.map_expected(…))`. En la plantilla: `ModerationGateway::review_content` devuelve `Result<ModerationVerdict, Failure<ModerationError>>`, y `CreateCommunityHandler` traduce `Unavailable` a la hoja `DependencyUnavailable` y un `Rejected` a la hoja `ContentRejected` |
| Falla técnica en un adaptador | `?` convierte cualquier `E: Error` en `Fault` (`impl From<E> for Fault`) |
| No encontrado | `find_*` devuelve `Option`; el handler decide si es `NotFound` o `Conflict` |
| Invariante roto / bug | `panic!` o `.expect("invariante: …")`. **`unwrap` está prohibido** (clippy `unwrap_used = "deny"`): no dice qué invariante se rompió. |
| Borde (ERR-4) | `match` exhaustivo, sin brazo comodín (tampoco sobre `Field`): `Expected(e)` → helper por hoja que devuelve `edge_http::Problem` con su `code`; `Unexpected(f)` → `edge_http::unexpected` (reporta + 500 genérico); `panic!` → `CatchPanicLayer` |
| Código estable para clientes | `Problem::conflict(…).with_code("COMMUNITIES_SLUG_ALREADY_EXISTS")`. Los códigos viven en `{ctx}-api/src/codes.rs`: son vocabulario de transporte |

**Por qué `Fault` no implementa `Error`.** Permite el `impl<E: Error> From<E> for Fault`
(mismo truco que `anyhow`), así `?` funciona con `sqlx::Error`, `io::Error`, etc.
dentro de los adaptadores. A la vez, `?` **no** convierte un error de dominio ni en
`Failure<E>` (no existe `From<ErrorDeDominio> for Failure<E>`) ni en `Fault`. Esto
último es así porque los errores de dominio no implementan `Error`, así que el
`impl From<E: Error>` no los alcanza. En los dos casos el compilador obliga a escribir
la traducción. Si alguien agrega `impl Error` a un error de dominio, el `?` hacia
`Fault` vuelve a compilar y el fallo de negocio termina como un 500 genérico. Por eso
un test de arquitectura lo impide (ERR-1), y otro hace lo mismo con los errores de
`application`: hojas, contratos `{Verb}{Noun}Error` y errores de los puertos. El compilador incluso sugiere ese `impl`
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
| Contrato del puerto | `community_repository_contract! { new_adapter: …, run: … }` y `moderation_gateway_contract! { new_adapter: \|scenario\| …, run: …, approves: …, rejects: …, unavailable: … }` contra el doble y contra cada adaptador | `application/src/testing.rs` (doble) y el archivo de cada adaptador |
| Adaptador | La batería de contrato, más lo propio de su tecnología (Postgres con testcontainers o similar) | crate de infraestructura |
| Transporte | `{Ctx}Api { … }.protected().oneshot(request)` con un `Stub{Verb}{Noun}UseCase` (o `Spy…` si el test afirma lo que recibió); la identidad se inyecta con `request.extension(AuthenticatedAccount { … })` | `contexts/{ctx}/api/tests/` |
| Arquitectura | `cargo test -p architecture` | `tools/architecture/tests/` |

- **Dobles** en `application/src/testing.rs`, compilados con `cfg(any(test, feature = "testing"))`.
  Otras capas los usan con `[dev-dependencies] {ctx}-application = { workspace = true, features = ["testing"] }`.
  Viven en `application` porque sus tests no ven `infrastructure` (HEX-1).
- **Todos los dobles** se nombran `{Prefijo}{Concepto}Adapter`, con un prefijo de la lista
  cerrada de TDD-2: `Fake` (con estado), `Fixed` o `Stub` (respuesta fija) y `Spy` (registra
  llamadas). Vale también para los locales a un archivo de test: `FakeCommunityAdapter`,
  `FixedIdGeneratorAdapter` y `SpyFaultReporterAdapter` (en `edge/http` y en los tests de `api`).
  Los dobles de puertos entrantes llevan el prefijo más el nombre del puerto, sin `Adapter`:
  `StubCreateCommunityUseCase` en `communities-api`, `SpyVerifyAccessTokenUseCase` en `iam-api`
  y `SpyCommunityEventSubscriber` en el despachador (`tdd_2_inbound_doubles_*`).
  `hex_5_adapters_*` recorre `src/` y `tests/` de todos los crates. Es de test todo `tests/`,
  lo que está bajo un `cfg` que menciona `test` (incluido `testing`) y lo que está dentro de
  una función `#[test]`.
- **Fallas por operación.** `FakeCommunityAdapter::failing()` hace fallar todas las operaciones;
  `FakeCommunityAdapter::empty().failing_on(CommunityOperation::Save)`, solo esa, y las demás
  siguen el contrato (`CommunityOperation` es un `enum` con una variante por método del puerto).
  `CreateCommunityHandler` llama a `find_by_slug` y después a `save`: con `failing()` el test
  nunca llega a `save`. Cada llamada a un puerto que puede fallar tiene su
  `fails_with_unexpected_when_<operación>_fails` (TDD-2); `SpyCommunityEventAdapter::failing()`
  cubre `publish`. Un caso de uso que llama a una sola operación del puerto
  (`RecordCommunityHistorySubscriber` solo llama a `append`) no necesita `failing_on`.
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
  (`new_adapter: impl Fn(Scenario) -> R`, con una variante de `Scenario` por resultado,
  incluida una por error declarado), y la batería tiene un caso por variante.

**Puerto con errores declarados.** `testing/contracts/moderation_gateway.rs` define
`Scenario { Approves, Rejects, Unavailable }` y un caso por cada uno. `scenario_of` hace un
`match` exhaustivo sobre `ModerationError`: una variante nueva no compila sin su escenario. La
macro exige un campo por caso, `supported` o `unsupported("motivo")`:

```rust
// application/src/testing.rs: el doble produce los tres escenarios
crate::moderation_gateway_contract! {
    new_adapter: FakeModerationAdapter::new,
    run: block_on,
    approves: supported,
    rejects: supported,
    unavailable: supported,
}

// infrastructure/src/no_op_moderation.rs: el NoOp solo aprueba
communities_application::moderation_gateway_contract! {
    new_adapter: |_| NoOpModerationAdapter,
    run: block_on,
    approves: supported,
    rejects: unsupported("NoOp aprueba todo: no consulta un servicio que pueda rechazar"),
    unavailable: unsupported("NoOp no hace E/S: no hay servicio que pueda no responder"),
}
```

- **Por qué todos los campos son obligatorios.** Un adaptador que no puede producir un
  escenario lo declara en su invocación, con el motivo, y nunca se salta un caso en silencio:
  omitir un campo no compila. Por lo mismo, un caso nuevo en la batería obliga a cada adaptador
  a decidir.
- **Qué genera `unsupported`.** El caso queda como `#[ignore = "motivo"]`: `cargo test` lo
  muestra como `ignored, motivo`, y `--include-ignored` lo corre y muestra por qué no pasa.
  Un `supported` que el adaptador no cumple falla como cualquier caso.

**Puerto sin errores declarados cuyo resultado depende de la entrada.** Con un verificador
de tokens, el escenario no lo fija la fábrica sino el token. `access_token_service_contract!`
recibe, además de `new_adapter`, un `issue: |adapter, subject| …` que emite un token válido
con la misma clave. Los casos lo usan para el válido y para el alterado. El doble emite tokens
propios (`FakeAccessTokenAdapter::issue`); el adaptador JWT los firma con `jsonwebtoken` en su
test. Lo propio de la tecnología (vencido, otra clave) va en el test del adaptador, no en la
batería.

**Puerto nuevo:**

1. Escribe `testing/contracts/{puerto}.rs`, con un caso por comportamiento del que dependan
   los handlers y uno por variante de error declarada. Agrega su macro y el `pub mod`.
2. Invoca la macro en `testing.rs`, contra el doble, y ve los casos en rojo.
3. Implementa el doble hasta que pasen. Después, cada adaptador invoca la macro en su archivo.

- En crates de test de integración, `#![allow(clippy::unwrap_used)]` al inicio del archivo.

**Puerto todavía síncrono (adopción).** En una adopción, un puerto puede seguir devolviendo
`Result<T, Fault>` sin `BoxFuture` hasta su *rebanada de asincronía*: la rebanada del plan de
adopción que lo pasa a `BoxFuture`. Es una desviación de RS-3 y se registra como tal en el
inventario de `docs/harness-adoption.md`. Su batería nace igual con la forma final: casos
`pub async fn`, helpers `async fn` y macro con `run`, invocada con `run: block_on`. Las
llamadas al puerto, en los helpers, van sin `.await`:

```rust
// testing/contracts/community_repository.rs, con el puerto síncrono
#![expect(
    clippy::unused_async,
    reason = "puerto todavía síncrono: se quita en su rebanada de asincronía"
)]

async fn save(adapter: &impl CommunityRepository, community: &Community) {
    adapter
        .save(community)
        .expect("contrato: save no falla con el adaptador sano");
}

pub async fn finds_a_saved_community_by_slug<R: CommunityRepository>(new_adapter: impl Fn() -> R) {
    let adapter = new_adapter();
    let saved = community("c-1", "rust-chile");
    save(&adapter, &saved).await;
    // …
}
```

Cuando el puerto pase a `BoxFuture`, cambian los helpers (cada llamada al puerto gana su
`.await`) y desaparece el `#![expect]`. Mientras algún `async fn` no espere nada, el `expect`
se cumple con el clippy por defecto. Cuando ya no queda ninguno, `unfulfilled_lint_expectations`
falla con `-D warnings` y obliga a borrarlo. La macro y las invocaciones del doble y de los
adaptadores que resuelven de inmediato (en memoria) no cambian. Un adaptador con E/S que hoy
bloquea pasa con `run: block_on`; en la rebanada de asincronía cambia a un runtime real (la
forma con `tokio` de arriba). Antes no: si el adaptador síncrono usa por dentro un runtime de
`tokio`, un `run` de `tokio` alrededor entra en pánico (no se puede iniciar un runtime dentro de
otro). Una batería síncrona sin `run` obliga a reescribir, en esa rebanada, los casos, la macro y
cada invocación.

---

## Enforcement (RS-4, VER-1)

| Regla | Mecanismo | Falla en |
|-------|-----------|----------|
| HEX-1 dependencias hacia adentro, `api` solo ve el `application` de su contexto y `edge`, sin cruces entre contextos | `tests/dependencies.rs::hex_1_*` (lee los `Cargo.toml`) + Cargo | `cargo test` |
| HEX-1 adaptador driving: `api` (y `infrastructure/http` en legacy) no nombra el dominio ni `ports::outbound`, ni en `use` (también por un módulo padre importado o un glob) ni en rutas del código. En legacy, Cargo no lo impide porque comparten crate. Un `api` central ve todos los contextos. No lee dentro de macros | `tests/naming.rs::hex_1_driving_adapters_see_only_use_cases` (`syn`) | `cargo test` |
| HEX-2 núcleo solo con `std` + kernel; TDD-2 sus tests tampoco usan runtime ni framework (lee también `[dev-dependencies]`) | `tests/dependencies.rs::hex_2_core_*` | `cargo test` |
| HEX-2 el código de kernel, domain y application no nombra un crate externo: `use` (también en grupo o con alias), `extern crate`, o el primer segmento de una ruta, también dentro de macros, de `derive(…)` y de `cfg_attr(…)`. Lo local (módulos e ítems del archivo, lo importado de `crate`, `self` o `super`) y los crates del workspace, también renombrados, no cuentan. Clave por archivo, una violación por crate: con el paquete ya en la línea base, un `use` nuevo en otro archivo falla. Lee `src/` y `tests/` de cada capa; no lee `build.rs`, `benches/` ni `examples/`, ni el `tests/` de un contexto legacy, que no es de una capa | `tests/dependencies.rs::hex_2_core_imports_*` (`syn`) | `cargo test` |
| HEX-2 frameworks de servidor solo en `api`, `edge` y bootstrap | `tests/dependencies.rs::hex_2_server_*` | `cargo test` |
| HEX-3 `domain` no ve `application` | Cargo (no está declarado) + `hex_1_*` | compilación |
| HEX-3 `domain` no declara puertos: no nombra `BoxFuture`, `Fault` ni `Failure` del kernel (por ruta, por nombre importado o con alias, también dentro de macros; `use kernel::*` falla porque no se puede verificar). El criterio es semántico, no de nombres: un puerto implica E/S | `tests/ports.rs::hex_3_*` (`syn` + tokens) | `cargo test` |
| HEX-4 `UseCase` de un solo `execute`; implementaciones `*Handler`; `api` nunca nombra un `*Handler` | `tests/naming.rs::hex_4_*` (`syn`) | `cargo test` |
| TDD-2 dobles de puertos entrantes `{Fake\|Fixed\|Stub\|Spy}{Puerto}`, en código de test de todos los crates | `tests/naming.rs::tdd_2_inbound_doubles_are_named_by_prefix_and_port` (`syn`) | `cargo test` |
| HEX-4 puerto entrante: `*UseCase` con solo `execute`, o `*EventSubscriber` con solo `on`; implementaciones `*Subscriber` (no `*EventSubscriber`) en `application/src/subscribers/`, sin implementar un `UseCase`. El `EventSubscriber` puede recibir el evento de dominio | `tests/naming.rs::hex_4_inbound_ports_are_single_method_use_cases`, `hex_4_event_subscribers_are_named_and_placed`, `hex_4_use_case_contracts_do_not_expose_domain_types` | `cargo test` |
| HEX-4 `*Workflow`/`*WorkflowError` solo en `application/src/workflows/`, sin implementar un `UseCase`, y `api` nunca los nombra; ERR-1 los cubre también (`impl Error` prohibido) | `tests/naming.rs::hex_4_workflows_are_internal_to_application`, `tests/errors.rs::err_1_application_*` | `cargo test` |
| HEX-4 el contrato no expone `domain` (tampoco el del kernel: `kernel::domain`, o `shared::domain` en legacy): campos públicos y variantes de `*Command`/`*Query`/`*Response`/`*Error`, firmas de `ports/inbound` y `pub use` del dominio en `application`, con ruta completa o con nombre importado | `tests/naming.rs::hex_4_use_case_contracts_do_not_expose_domain_types` (`syn`). Cargo no basta: `api` puede usar un valor de dominio que le llega por un campo público sin declarar el crate | `cargo test` |
| HEX-4 / ERR-2 firma del caso de uso: `execute(&self, {Verb}{Noun}Command \| {Verb}{Noun}Query)` devuelve `Result<{Verb}{Noun}Response, Failure<{Verb}{Noun}Error>>` (o `Fault` sin árbol A), directo o en un `BoxFuture`. Los tres tipos están definidos en `commands/{verb_noun}/` o `queries/{verb_noun}/`. La identidad del llamador va en el comando, no como segundo argumento. Sin alias de tipo en la firma. Sin árbol A, `Result<…, Fault>` y ningún `{Verb}{Noun}Error`. Los nombres con siglas van agrupados en el directorio: `VerifyHTTPToken` → `verify_http_token` | `tests/naming.rs::hex_4_use_case_signatures_are_their_own_contract` (`syn`) | `cargo test` |
| HEX-5 puertos salientes con sufijo de rol (incluido `Publisher`) o un nombre propio de la lista (`UnitOfWork`), nunca `Port` | `tests/naming.rs::hex_5_outbound_ports_are_named_by_role` | `cargo test` |
| HEX-5 / TDD-2 adaptadores `{Prefijo}{Concepto}Adapter`, en todo el workspace (`src/` y `tests/`, también `#[cfg(test)]`): todo tipo que implementa un puerto de `ports/outbound` o una capacidad del kernel. Concepto es el puerto sin su rol, o la capacidad completa. En código de test, el prefijo es `Fake`, `Fixed`, `Stub` o `Spy`. En producción, el prefijo no está vacío, no empieza con uno de esos, y el adaptador vive en infraestructura (`contexts/{ctx}/infrastructure` o `infrastructure/*`) | `tests/naming.rs::hex_5_adapters_are_named_by_technology_and_concept` (`syn`) | `cargo test` |
| HEX-8 el kernel no declara dependencias (ni de test ni por `target`), sus traits públicos son marcadores de modelo o capacidades de la lista cerrada, y los marcadores no declaran métodos, tipos ni constantes (`pub trait AggregateRoot {}`; admiten supertraits estándar u otro marcador, nunca una capacidad) | `tests/kernel.rs::hex_8_*` (`toml` + `syn`) | `cargo test` |
| HEX-6 vocabulario de DTOs y `{Caso}Http*` en `api` | `tests/naming.rs::hex_6_*` | `cargo test` |
| HEX-6 tipos de un puerto saliente: todo tipo definido en `application` que aparece en su firma vive en `ports/outbound/dtos/` (también sus errores declarados; `dtos/` o `dtos.rs`). Un nombre del dominio, del kernel o de `std` no cuenta aunque `application` tenga un homónimo | `tests/naming.rs::hex_6_outbound_port_types_live_in_dtos` (`syn`) | `cargo test` |
| HEX-7 ningún tipo de `domain` ni de `application` fuera de `ports/outbound` se llama `…Service` (structs, enums y traits, también en código de test) | `tests/naming.rs::hex_7_service_suffix_is_only_an_outbound_role` (`syn`) | `cargo test` |
| ERR-1/RS-2 cada `expect` de producción empieza con `"invariante:"` (o `"pendiente:"`, la forma transitoria de una firma que pasa a ser falible, TDD-1). También `expect_err` y raw strings. Recorre el kernel, los contextos, `edge/*`, `infrastructure/*` y `apps/*`, no `tools/*`, y omite lo que está bajo un `cfg` que menciona `test` (los tests y los dobles de `testing`). Un método propio llamado `expect` también cuenta: renombrarlo. Que el invariante lo sea y que `"pendiente:"` no sobreviva, lo revisa el revisor | `tests/output.rs::rs_2_expect_names_the_invariant` (tokens) | `cargo test` |
| ERR-1/RS-2 ningún error de dominio implementa `std::error::Error` (tampoco dentro de un `macro_rules!`) | `tests/errors.rs::err_1_domain_errors_do_not_implement_std_error` (`syn`) + compilador: `?` no convierte el error en `Fault` | `cargo test` |
| ERR-1 en `application` tampoco: las hojas de `errors.rs`, los `*Error` de `commands/` y `queries/` (`{Verb}{Noun}Error`), y los errores de los puertos (los `*Error` de `ports/` y todo `E` de un `Failure<E>` en `ports/`). El `impl` se busca en cualquier archivo del crate, y uno generado por macro sobre `$tipo` cuenta | `tests/errors.rs::err_1_application_errors_do_not_implement_std_error` (`syn`) | `cargo test` |
| ERR-4 el núcleo no escribe en la salida: `domain` y `application` no invocan `println!`, `eprintln!`, `print!`, `eprint!` ni `dbg!` (también con ruta `std::…!`, dentro de otra macro o de un `macro_rules!`). Exento lo que exige `test`: `#[cfg(test)]`, `#[cfg(all(test, …))]`, `#![cfg(test)]` y los archivos de un `#[cfg(test)] mod x;`. `testing` (`cfg(any(test, feature = "testing"))`) no está exento. Los crates de logging ya los impide `hex_2_core_*` | `tests/output.rs::err_4_*` (`syn` + tokens) | `cargo test` |
| ERR-4 el `fold` de un adaptador driving nombra cada variante de los enums de `application` y del kernel (el contrato, `Field`, `Failure`). Un comodín (`_`, una variable, `..`) cuenta en la posición donde otro brazo del mismo `match` nombra un enum, también anidado (`Err(_)`, `Failure::Expected(_)`, `Field::Card(_)`), dentro de campos de struct, referencias, paréntesis y tuplas. Un `if let`, `let … else` o `matches!` sobre uno de esos enums también cuenta. Límites: el enum se reconoce por nombre (uno de `api` con el mismo nombre se tomaría por contrato); `Hoja(_)` solo, sin otro brazo que nombre su contenido, no cuenta; no mira `#[cfg(test)]` ni un `match` sobre texto | `tests/errors.rs::err_4_driving_folds_*` (`syn`) | `cargo test` | `tests/errors.rs::err_4_driving_folds_*` (`syn`) | `cargo test` |
| `unwrap` prohibido; `unsafe` prohibido | `[workspace.lints]` | `cargo clippy -D warnings` |
| HEX-1…6, HEX-7 (nombres), HEX-8, ERR-1, ERR-4 mientras el agente edita | Hook `PostToolUse` de craft-backend: `rustfmt` + `cargo test -p architecture` | cada edición de `.rs`/`Cargo.toml` (~1 s) |
| Todo lo anterior antes de cerrar | Hook `Stop` de craft-backend: `clippy --no-deps -D warnings` sobre los crates de la arquitectura + `cargo test --workspace` | al terminar, si hay cambios Rust sin commit |
| TDD-2 / ERR-3 el doble y los adaptadores cumplen el mismo contrato del puerto | Batería de contrato (`application::testing::contracts`) | `cargo test` |
| HEX-7, ERR-1…3, TDD-1 (test antes o junto a la producción en el diff y en el orden de commits) | `architecture-reviewer`. Los hooks garantizan verde, no rojo primero | revisión |

**Sin material no hay verde (`ARCH_REQUIRE_MATERIAL`).** Un test de arquitectura que no
encuentra nada sobre qué actuar es indistinguible de uno que verificó todo. Cada test informa
cuántos elementos revisó por conjunto (crates de una capa, archivos, tipos de una categoría;
visible con `cargo test -p architecture -- --nocapture`), a través de `assert_no_violations`
de `tests/common/mod.rs`. Con `ARCH_REQUIRE_MATERIAL=1`, un conjunto vacío es un fallo que
nombra la regla y lo que faltó. Sin la variable pasa, porque en un proyecto recién creado
puede ser legítimo que todavía no haya, por ejemplo, `queries/`. El CI del harness la activa
sobre la plantilla; el CI mínimo de abajo, el que hereda un proyecto, no.

**Exenciones de material (`material-exempt.txt`).** Un proyecto que activa la variable puede
tener un conjunto vacío para siempre: sin eventos, no hay implementaciones de un
`EventSubscriber`; con todos los puertos en `Fault`, no hay errores declarados de puertos. En
vez de apagar la variable, se exime ese conjunto, uno por línea:

```
hex_4_event_subscribers_are_named_and_placed | implementaciones de un EventSubscriber en application (fuera de tests) | sin eventos de dominio (decisión D4)
```

El conjunto es el texto exacto que el test informa (`revisó 0 …` con `--nocapture`), y el
motivo es obligatorio. La exención funciona como la línea base, solo se achica: si el conjunto
deja de estar vacío, el test eximido pide borrarla (falla con o sin la variable), y también
falla si el conjunto no es suyo. `ratchet.rs::ver_1_material_exemptions_name_tests` falla si
la línea no tiene motivo, si se repite o si el test no existe o no revisa material (no llama a
`assert_no_violations`). El hook `pre-edit` bloquea al agente cualquier exención nueva,
también con `CRAFT_BACKEND_BASELINE_INIT=1`: la escribe el usuario a mano. La plantilla la trae
vacía. Si una versión del harness renombra un test o el texto de un conjunto, el CHANGELOG lo
dice con el mapeo viejo → nuevo, como con las claves de `baseline.txt`.

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

### Formato al copiar de la plantilla

La plantilla publica `rustfmt.toml` (`style_edition = "2024"`, estable). En un
proyecto nuevo, copiar un archivo tal cual da un archivo idéntico. Si el
proyecto ya tiene su `rustfmt.toml` (por ejemplo, con opciones de nightly como
`imports_granularity`), se conserva el del proyecto: después de copiar, `cargo
fmt` deja la copia distinta solo en formato (orden y agrupación de `use`), no en
contenido. Al comparar con la plantilla, se compara después de `cargo fmt`.

### Trinquete: `tools/architecture/baseline.txt`

Al adoptar el harness en un proyecto existente, los tests de arquitectura
fallan por violaciones que no se pueden corregir de una vez. La línea base las
registra y el trinquete impide que crezcan:

1. **Se llena con la auditoría inicial** (skill `craft-backend:adoption`) y solo
   vuelve a llenarse al sincronizar `tools/architecture` con una versión del harness
   que cambia las claves (`/craft-backend:adoption sincronizar <tag>`, en un PR
   propio): una línea
   `<test> <clave> [n]` por clave distinta. `<test>` es el nombre de la función
   del test. `<clave>` es lo que el mensaje pone antes del primer `:`: la ruta
   del archivo, o `ruta/Cargo.toml#sección.paquete` en los tests de
   dependencias (por ejemplo `#dev-dependencies.tokio`). `n` es cuántas
   violaciones de esa clave se admiten, 1 si se omite.
   Las entradas no se escriben a mano. Con la línea base vaciada hasta la cabecera:
   ```
   ARCH_BASELINE_PRINT=1 cargo test -p architecture --no-fail-fast -- --nocapture 2>&1 \
     | grep -o 'baseline: .*' | sed 's/^baseline: //' | sort > /tmp/baseline-entries.txt
   ```
   y las entradas se agregan a `baseline.txt` con la herramienta de edición (Write), no
   con `>>`: así las ve el hook `pre-edit`, que exige `CRAFT_BACKEND_BASELINE_INIT=1`.
   Cada test imprime `baseline: <test> <clave> [n]` por clave, con el conteo ya
   calculado, en una sola escritura (`ratchet.rs::ver_1_baseline_lines_*` y
   `ver_1_baseline_print_only_with_the_variable`). `grep -o` y no `sed -n 's/^…//p'`:
   con un solo hilo de test, libtest deja `test <nombre> ... ` en la misma línea. Una
   clave de `[target.'cfg(…)'.dependencies]` va sin espacios. `--no-fail-fast` es
   obligatorio: sin él, `cargo test` se corta en el primer binario de test que
   falla y las entradas de los demás no aparecen. La variable no cambia el
   resultado: el test falla igual, así que olvidarla activa no apaga el trinquete.
2. **Solo se achica.** El test falla si aparece una clave que no está en la
   línea base o si una clave supera su `n`. También falla si una entrada ya no
   viola (`borrar de baseline.txt: …`) o viola menos (`actualizar en
   baseline.txt: … n`). En un refactor que corrige una violación, el rojo va
   primero: se borra o baja la entrada, se ve fallar el test con esa violación
   y se corrige hasta verde. Que nadie *agregue* una línea
   lo bloquea el hook `PreToolUse` de craft-backend en las ediciones del
   agente; el llenado inicial se hace con `CRAFT_BACKEND_BASELINE_INIT=1`, exportada
   **antes de abrir la sesión** (`CRAFT_BACKEND_BASELINE_INIT=1 claude`): el hook
   hereda el entorno de Claude Code al arrancar, y exportarla a mitad de sesión no
   llega. Antes de vaciar la línea base, compruébala (`echo
   $CRAFT_BACKEND_BASELINE_INIT` debe dar `1`); el hook `SessionStart` también avisa
   que la sesión tiene el trinquete abierto. Una edición por shell o a mano la ve el
   revisor del PR.
3. **Corregir en el lugar y después mover.** La línea base no sigue los
   movimientos: mover un archivo con entradas da a la vez una violación nueva y
   una entrada para borrar. Primero se corrige en la ruta vieja y se borra la
   entrada; después se mueve. Si las dos cosas aparecen juntas, el test lo
   sugiere.
4. **Los ids son verificados.** `ver_1_baseline_ids_match_tests` exige que cada
   llamada a `assert_no_violations` pase el nombre de su función y que cada
   `<test>` de la línea base exista.

En un proyecto creado desde la plantilla, `baseline.txt` queda solo con la
cabecera.

### Contextos legacy

Un workspace puede tener contextos ya separados por capas y otros *legacy* (un
solo crate en `contexts/{ctx}/Cargo.toml`, con las capas como módulos) mientras
se migra. `tools/architecture/tests/common/mod.rs` trae un modo legacy:

- **Kernel:** si no hay `kernel/`, el kernel es `shared/` (`LEGACY_KERNEL_DIR`;
  cámbialo si el proyecto le da otro nombre).
- **Capas virtuales:** cada módulo `src/{domain,application,infrastructure}` de
  un crate legacy (directorio, archivo `src/domain.rs`, o ambos) se clasifica
  como esa capa, y `src/infrastructure/http` como Api, que se asume driving y
  no se lee también como Infrastructure. `lib.rs`, `main.rs` y los módulos con
  otro nombre (`src/api/`, `src/http/`) no se leen: ajusta `LEGACY_MODULES` si
  el proyecto usa otros. Todas comparten el manifiesto, así que lo que declara cuenta en cada
  capa: un contexto legacy con `axum` viola HEX-2 en domain y en application
  hasta que se separe. Esas violaciones van a la línea base (la misma clave
  `ruta/Cargo.toml#…` con su conteo). Como el manifiesto no distingue capas,
  `hex_2_core_imports_only_std_and_the_kernel` cuenta además cada archivo de
  domain, application y del kernel que nombra el crate: un `use` nuevo en
  otro archivo falla aunque el paquete ya tenga entrada.
- **HEX-4 actúa en legacy:** en application, el dominio es `crate::domain`
  (también importado, por nombre o con `super::…::domain`), así que un
  contrato que lo expone se detecta antes de separar el crate. Las reglas de
  ubicación (workflows, subscribers, el rol `Service` en `ports/outbound`)
  resuelven las rutas por capa: `src/workflows` es `src/application/workflows`.
  HEX-3 reconoce el kernel legacy (`shared::Fault`).
- **Límites conocidos:** un contexto legacy no puede depender de `edge/*`
  (HEX-1 prohíbe `Domain → Edge`, y el manifiesto es compartido). Un crate
  migrado que dependa de uno legacy falla en HEX-1; el mensaje nombra una de
  las capas virtuales, porque comparten el nombre del crate (también falla un
  `apps/api` central que monte un contexto legacy). Un reexporte del dominio
  desde `lib.rs` (`pub use domain::Account` y luego `crate::Account`) no se
  detecta.
- **Se borra al terminar:** cuando no queda ningún contexto legacy y el kernel
  está en `kernel/`, se borra todo lo marcado `legacy:` en
  `tools/architecture/tests` (`grep -rn 'legacy:'`), más `tests/legacy.rs` y
  `tests/fixtures/legacy/`, y los tests siguen en verde.
- El gate de Stop aplica clippy solo a los crates de la arquitectura (`--no-deps -p …`);
  los tests sí corren en todo el workspace. Así el código legacy con warnings no
  bloquea, pero un test legacy roto sí.
- Con contextos legacy, lista los miembros explícitamente
  (`"contexts/iam", "contexts/communities/*"`): el glob `contexts/*/*` tomaría
  `contexts/iam/src` como miembro y `exclude` no lo evita.

### `tests/` de los crates

Los `tests/` de un crate no definen tipos de producción, así que las reglas de
contratos, errores, salida, puertos y vocabulario (HEX-3, HEX-4 de contratos,
HEX-6, ERR-1, ERR-4) leen solo `src/`. Lo que sí rige en `tests/`: sus
dependencias (`[dev-dependencies]`, por HEX-2), los nombres de adaptadores y
dobles (HEX-5, TDD-2) y que los tests de api no llamen a handlers ni a
workflows (HEX-4). En un contexto legacy, `tests/` no
pertenece a ninguna capa y no se lee.

## Violaciones típicas en Rust

| En vez de | Usar |
|-----------|------|
| Un crate por contexto con capas como módulos | Un crate por capa (RS-1) |
| Rutas HTTP dentro de `infrastructure` junto a la persistencia | Crate `{ctx}-api` separado; `api` e `infrastructure` no se ven (HEX-1) |
| `trait DomainError` o `ApplicationError` en el kernel (marcadores de Java) | Nada: el `enum` `{Verb}{Noun}Error` ya cierra el contrato. En Rust el kernel no declara marcadores de error (HEX-8) |
| `uuid`, `serde`, `chrono` en `domain`/`application` | Solo `std` + kernel; la implementación va en infraestructura |
| `trait CommunityRepositoryPort` | `trait CommunityRepository` |
| `PasswordService` en el dominio | `PasswordPolicy` (HEX-7) |
| `trait TokenVerifier` en un `shared` que ven todos los contextos | `VerifyAccessTokenUseCase` en `iam` y su middleware en `iam-api`, que el composition root aplica (HEX-1) |
| `JWT_SECRET` con valor por defecto, o `#[derive(Debug)]` en la configuración | Obligatorio al arrancar; `Config` sin `Debug` (ERR-4) |
| Todo el cableado en un `main.rs` que crece con cada contexto | Un `contexts/{ctx}.rs` por contexto que devuelve su `{Ctx}Api` |
| `AccountFactory::create` / `::reconstitute`, que reciben `String` | `Account::create` / `Account::restore`, que reciben value objects; el adaptador traduce los `String` persistidos |
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
