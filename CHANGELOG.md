# Changelog

## 0.3.0 — 2026-11-12

### Agregado
- `stacks/rust/`: guía de traducción con las decisiones RS-1 a RS-4.
- `stacks/rust/template/`: workspace ejecutable (kernel, contexto `communities`,
  `api`, `server`, `tools/architecture`). 33 tests; los 9 tests de arquitectura
  se verificaron inyectando una violación por regla.
- Skill `rust-stack`, que enlaza la guía y la plantilla.

### Decisiones
- **RS-1, un crate por capa agrupado por contexto**
  (`contexts/{ctx}/{domain,application,infrastructure}`). Cargo hace cumplir la
  dirección de dependencias al compilar.
- **RS-2, árbol B en Rust:** `Fault` opaco dentro de `Failure<E>`, propagado con
  `?`; `panic!` solo para invariantes, capturado con `CatchPanicLayer`; `unwrap`
  prohibido por clippy. Cierra la decisión abierta de `errors.md`.
- **RS-3, puertos async con `BoxFuture`** del kernel: solo `std` y compatible con `dyn`.
- **RS-4, tests de arquitectura con `toml` + `syn`** en lugar de `cargo-deny`: un
  solo mecanismo cubre dependencias internas, crates externos por capa y nombres.
  Cierra la decisión abierta de `verification.md` para Rust.
- `IdGenerator` en el kernel, junto a los helpers genéricos de HEX-8, para que
  `uuid` no entre al dominio.

## 0.2.0 — 2026-11-11

### Cambiado
- Principios separados por tema, con IDs propios: `hexagonal.md` (HEX-1–8),
  `errors.md` (ERR-1–4), `tdd.md` (TDD-1–2), `verification.md` (VER-1).
- Tabla de traducción por stack movida a `stacks/README.md`.
- Skills separadas: `hexagonal-architecture`, `error-handling`, `tdd`.
- `architecture-reviewer` precarga las tres skills y cita los nuevos IDs.

### Decisiones
- **Puertos en `ports/inbound` y `ports/outbound`** (antes `ports.in`/`ports.out`):
  `in` es palabra reservada en Kotlin y Rust.
- **Tipos de transporte `{Caso}HttpRequest` / `{Caso}HttpResponse`**: elimina la
  colisión de `Response` (caso de uso) y de `Request` (puerto saliente) con el
  vocabulario de `api` (HEX-6).
- **Disposición:** handlers y su contrato en `application/commands/{caso}` y
  `application/queries/{caso}`; la interfaz `UseCase` en `application/ports/inbound`.
- **`api` solo depende de `application`** (HEX-1). Consecuencias: el contrato del
  caso de uso no expone tipos de dominio (HEX-4) y las hojas de error compartidas
  viven en `application/errors` (ERR-2).
- **Adaptadores sin el sufijo de rol** (HEX-5): `PaymentRepository` →
  `PostgresPaymentAdapter`. La tecnología ya implica el rol.

## 0.1.0 — 2026-11-11

- Primer corte vertical: principio hexagonal P1–P14, skill, subagente revisor y
  marketplace.
