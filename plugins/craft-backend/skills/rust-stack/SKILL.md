---
name: rust-stack
description: Cómo se expresan los principios craft-backend (hexagonal, errores, TDD) en Rust con un workspace Cargo. Usar SIEMPRE que se trabaje en un backend Rust con crates por capa (`kernel/`, `contexts/*/domain`, `contexts/*/application`, `contexts/*/infrastructure`, `contexts/*/api`, `edge/http`, `apps/server`), al crear un contexto, crate, caso de uso, puerto, adaptador, error o test en Rust, o al revisar un `Cargo.toml`.
---

# Rust (craft-backend)

Complementa `hexagonal-architecture`, `error-handling` y `tdd`: esas skills
dicen **qué**; esta dice **cómo en Rust**.

- `references/rust.md`: guía completa (decisiones RS-1 a RS-4, tablas de traducción).
- `references/template/`: workspace de referencia que compila y pasa todos los tests.
  **Antes de crear algo nuevo, copia la forma del archivo equivalente en
  `contexts/communities/`.**

## Decisiones del stack

- **RS-1: un crate por capa**, en `contexts/{ctx}/{domain,application,infrastructure,api}`.
  El crate se nombra `{ctx}-{capa}`. `{ctx}-api` solo ve su `{ctx}-application`,
  `edge-http` y el kernel. `edge/http` (Problem Details, `AuthenticatedAccount`,
  `unexpected`) es lo único compartido del borde. `apps/server` (o `app/`) es el
  composition root y compone las rutas. No hay dependencias entre contextos.
- **RS-2: árbol B = `kernel::Fault`**, propagado con `?`. Firmas:
  `Result<T, Failure<{Verb}{Noun}Error>>` en casos de uso, `Result<T, Fault>` en
  puertos sin errores declarados. `panic!`/`expect("invariante: …")` solo para bugs.
  **`unwrap` prohibido.** Los errores de dominio no implementan `std::error::Error`:
  así `?` no los convierte en `Fault`. La conversión a `Fault` se escribe a mano y
  solo para datos persistidos corruptos.
- **RS-3: puertos async con `kernel::BoxFuture`**. Nunca `async-trait` en el núcleo.
  Firma canónica: un único lifetime `'a` para `self` y para los argumentos
  prestados, así el future puede tomar prestado todo sin clonar:
  ```rust
  fn save<'a>(&'a self, community: &'a Community) -> BoxFuture<'a, Result<(), Fault>>;
  ```
  Puertos entrantes con argumento por valor: `fn execute(&self, command: C) -> BoxFuture<'_, …>`.
  Implementación: `Box::pin(async move { … })`, o `Box::pin(ready(x))` si es síncrona.
- **RS-4: `cargo test -p architecture`** verifica dependencias y nombres. Si falla,
  corrige el código, **no el test**.

## Para un caso de uso nuevo (orden TDD)

1. `domain`: value objects/comportamiento + test puro, si hay reglas nuevas.
2. `application/src/commands/{caso}/` (o `queries/`): `command.rs`, `response.rs`,
   `error.rs` (enum con una variante por hoja de `errors.rs`) y el test en `handler.rs`.
   Rojo primero.
3. `application/src/ports/inbound/{caso}.rs`: `trait {Verb}{Noun}UseCase` con `execute`.
4. `handler.rs`: `{Verb}{Noun}Handler` con `Arc<dyn Puerto>`. Errores de dominio
   → `Failure::expected(Hoja { … })`, nunca envueltos.
5. Puertos salientes nuevos en `ports/outbound/` + su fake en `testing.rs`.
6. Adaptador en `infrastructure` (`{Tecnología}{Concepto}Adapter`) + su test.
7. `contexts/{ctx}/api`: `{Caso}HttpRequest`/`{Caso}HttpResponse`, `match`
   exhaustivo hacia un helper por hoja que devuelve `edge_http::Problem` con su
   `code` (`src/codes.rs`), y el test con `oneshot` y un `UseCase` stub.
8. Agrega el campo al `{Ctx}Api` y haz el wiring en el composition root.

## Antes de terminar

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
