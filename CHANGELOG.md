# Changelog

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
