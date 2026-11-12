# Changelog

## 0.5.0 — 2026-11-12

### Cambiado
- **RS-1:** los adaptadores driving viven por contexto (`contexts/{ctx}/api`,
  crate `{ctx}-api`). Lo transversal del borde HTTP (Problem Details,
  `AuthenticatedAccount`, `unexpected`) pasa al nuevo crate `edge/http`.
  `apps/api` desaparece de la plantilla; `apps/server` compone las rutas.
- Tests de arquitectura:
  - Nueva capa `Edge`. Un `{ctx}-api` solo ve su propio application.
  - `app/` en la raíz se acepta como composition root.
  - Los contextos legacy (un solo crate) se ignoran durante una migración.
  - Verificado inyectando 4 violaciones nuevas.
- ERR-4: Problem Details lleva un `code` estable como miembro de extensión.

### Agregado
- Sección "Migración incremental" en `stacks/rust`.

## 0.4.0 — 2026-11-12

### Agregado
- Hooks de `craft-backend` (`hooks/hooks.json` + `hooks/craft_hooks.py`):
  - `PostToolUse` sobre `Edit|Write|MultiEdit`: `rustfmt` del archivo y
    `cargo test -p architecture`. Las violaciones vuelven al agente con su ID.
  - `Stop`: si hay cambios Rust sin commit, `clippy -D warnings` y
    `cargo test --workspace`; bloquea una vez por turno (`stop_hook_active`).
  - Solo se activan en workspaces con `tools/architecture/Cargo.toml`.
  - Variables `CRAFT_BACKEND_HOOKS`, `CRAFT_BACKEND_STOP_GATE` y `CRAFT_BACKEND_HOOKS_LOG`.

### Verificado
- 8 casos simulados (archivo no Rust, fuera de workspace, edición limpia,
  violación, formateo, Stop sin cambios, Stop con test roto, `stop_hook_active`).
- Dos sesiones reales de Claude Code:
  - Se pidió explícitamente un `CommunityCachePort`. El hook lo rechazó por
    HEX-5 y el agente lo renombró a `CommunityCache`.
  - Con un bug sembrado en el dominio y una tarea sin relación, el gate de Stop
    bloqueó el cierre. El agente diagnosticó la causa y la reportó sin salirse
    del alcance pedido.

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
