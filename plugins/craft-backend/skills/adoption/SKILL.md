---
name: adoption
description: Adopción de craft-backend en un backend existente que todavía no cumple los principios. Cubre la auditoría inicial con su documento de estado (inventario con IDs estables, plan de rebanadas, decisiones, fricciones y registro de tokens), la línea base del trinquete y el ritual de cada rebanada, con su regla de ramas. Usar al empezar a adoptar el harness en un repo existente, al implementar una rebanada del plan (`/craft-backend:adoption R7`) o al replanificar. No aplica a un proyecto que nace de la plantilla.
---

# Adopción de craft-backend

Se usa junto con `hexagonal-architecture`, `error-handling`, `tdd` y la skill
del stack (en Rust, `rust-stack`). **Cárgalas antes de tocar código.**

- `references/adoption.md`: plantilla del documento de estado. Va a
  `docs/harness-adoption.md` del proyecto, y cada sesión lo lee entero.
- En Rust, el README del stack (`rust-stack`, `references/rust.md`, sección
  "Migración incremental") explica el trinquete y el modo legacy de
  `tools/architecture`.

## 1. Empezar (una vez)

1. Auditoría completa con `craft-backend:architecture-reviewer`, en solo
   lectura. Copia la plantilla y llena §1 a §4: un ID estable por hallazgo
   (`HEX-1.3`), con `archivo:línea` verificado.
2. Agrega `tools/architecture` desde la plantilla del stack. En Rust, con el
   modo legacy si hay un crate por contexto o un `shared/` como kernel.
3. Llena la línea base **una sola vez**, con lo que los tests de arquitectura
   reportan hoy. El hook bloquea las ediciones que la agrandan: abre esa sesión
   con `CRAFT_BACKEND_BASELINE_INIT=1` y ciérrala al terminar. Cada entrada se
   concilia con un hallazgo del inventario.
4. Escribe el plan (§5): rebanadas chicas, orden por dependencias, puntos de
   replanificación, y qué decisiones son del usuario (§5.7). Por defecto,
   **corregir en el lugar antes de mover**: el trinquete no sigue los
   movimientos.

## 2. Una rebanada (`R<n>` en los argumentos)

### Antes de empezar
1. `git switch main && git pull`. **Cada rebanada sale de `main`; nunca de la
   rama de otra rebanada.** No se apilan PRs.
2. Lee `docs/harness-adoption.md` entero. Confirma que las dependencias de la
   rebanada (§5.4) están hechas y su código está en `main`. Si no, detente y
   avisa.
3. Si depende de una decisión `D<n>` que no está decidida en §5.7, detente y
   pregunta.
4. Crea la rama desde `main`.

### Durante
- Solo el alcance de la rebanada (§5.3) y las instrucciones del usuario. No
  adelantes trabajo de otras.
- TDD: cada cambio de comportamiento empieza con un test en rojo, visto antes
  de implementar. En un refactor puro, el rojo es el trinquete (`borrar de
  baseline.txt` o `actualizar en baseline.txt`).
- La línea base **solo se achica**. Si un archivo que mueves tiene entradas, o
  aparece una violación nueva, detente y avisa. Nunca agregues líneas.
- La red de seguridad (tests de caracterización) no se modifica, salvo los
  cambios de contrato de §5.5 para esta rebanada.
- Commits en inglés con Conventional Commits.

### Al terminar
1. El CI local del stack en verde (en Rust: `cargo fmt --check`, `cargo clippy
   --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`).
2. Actualiza el inventario (`resuelto (<commit>)`), §5.3 (resultado), §5.6 y,
   si cambió un contrato, §5.5.
3. `architecture-reviewer` sobre el diff. Aplica lo de esta rebanada; lo demás
   se anota en su resultado de §5.3.
4. PR con resumen, tests nuevos con su rojo, entradas borradas de la línea base
   y cambios de contrato.
5. `gh pr view --json baseRefName` debe decir `main`. Si dice otra rama, corrige
   la base antes de seguir.
6. Fila en §5.8 con `Tokens: pendiente` hasta que el usuario pase el `/cost`.
7. Fricciones con el harness, en §6.
8. Reporta: comprobaciones previas, commits, rojo y verde, revisor y
   desviaciones respecto de lo pedido.

## 3. Replanificar

En cada punto de replanificación sigue §5.9 de la plantilla. Lo que cambie del
plan va en un PR de solo documentación antes de la rebanada siguiente.
