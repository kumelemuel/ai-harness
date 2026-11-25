---
name: adoption
description: Ritual de adopción de craft-backend en un backend existente (auditoría, documento de estado, línea base, rebanadas y sincronización con versiones nuevas del harness). Se invoca a mano con /craft-backend:adoption.
argument-hint: "[empezar | R<n> | replanificar | sincronizar <tag>]"
disable-model-invocation: true
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
   lectura. Dile en el encargo que el alcance es el repo entero, no un diff. Copia la plantilla y llena §1 a §4: un ID estable por hallazgo
   (`HEX-1.3`), con `archivo:línea` verificado.
2. **Red de seguridad.** Identifica los tests que fijan el contrato externo
   (HTTP u otro). Si no hay, la primera rebanada los escribe, antes de
   cualquier refactor: TDD-1 solo cubre un refactor con tests en verde que
   fijen el comportamiento. Anótalos en el documento (cabecera y §5.1).
3. Agrega `tools/architecture` desde la plantilla del stack. En Rust, con el
   modo legacy si hay un crate por contexto o un `shared/` como kernel.
4. Llena la línea base con lo que los tests de arquitectura reportan hoy. Es la
   única vez que se llena, salvo una sincronización con el harness (sección 4
   de esta skill). El hook bloquea las ediciones que la agrandan: el usuario
   abre esa sesión con la variable ya exportada (`CRAFT_BACKEND_BASELINE_INIT=1
   claude`) y la cierra al terminar. El hook la lee del entorno con que arrancó
   Claude Code: exportarla a mitad de sesión no sirve. **Antes de vaciar o tocar
   la línea base**, corre `echo $CRAFT_BACKEND_BASELINE_INIT`; si no da `1`,
   detente y pide al usuario que reabra con `CRAFT_BACKEND_BASELINE_INIT=1 claude
   --continue`. Cada entrada se concilia con un hallazgo del inventario.
5. Llena "Ritual de rebanada" en §5.1 con las convenciones del proyecto: lo que
   digan su `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md` o un comando de rebanada
   propio (`.claude/commands/`). Lo que no esté, pregúntalo.
6. Escribe el plan (§5): rebanadas chicas, orden por dependencias, puntos de
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
  de implementar. En un refactor de arquitectura, borra o baja su entrada de la línea base, confirma que el test falla con esa violación y corrige hasta verde.
- La línea base **solo se achica**. Si un archivo que mueves tiene entradas, o
  aparece una violación nueva, detente y avisa. Nunca agregues líneas.
- Al activar `ARCH_REQUIRE_MATERIAL=1`, un conjunto que quedará vacío para
  siempre (sin eventos, puertos sin errores declarados) se exime en
  `tools/architecture/material-exempt.txt`. Nunca lo escribes tú: propón la
  línea y su motivo, y el usuario la agrega a mano (el hook te bloquea).
- La red de seguridad (tests de caracterización) no se modifica, salvo los
  cambios de contrato de §5.5 para esta rebanada.
- Sigue el "Ritual de rebanada" de §5.1 (commits, limpieza). Si falta, usa la
  convención del repo y anota la fricción en §6.

### Al terminar
1. El CI local del stack en verde (en Rust: `cargo fmt --check`, `cargo clippy
   --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`).
2. Actualiza el inventario (`resuelto (<commit>)`), §5.3 (resultado), §5.6 y,
   si cambió un contrato, §5.5.
3. `architecture-reviewer` sobre el diff. Aplica lo de esta rebanada; lo demás
   se anota en su resultado de §5.3.
4. Push y PR si el usuario lo pidió o si el "Ritual de rebanada" de §5.1 dice
   que siempre. Si no, deja la rama con sus commits
   y pon el texto del PR en el reporte: resumen, tests nuevos con su rojo,
   entradas borradas de la línea base y cambios de contrato.
5. Si hay PR, su base debe ser `main` (en GitHub, `gh pr view --json
   baseRefName`). Si es otra rama, corrige la base antes de seguir.
6. Fila en §5.8 con el número de PR y `Tokens: pendiente` hasta que el usuario
   pase el `/cost`.
7. Fricciones con el harness, en §6.
8. Reporta: comprobaciones previas, commits, rojo y verde, revisor y
   desviaciones respecto de lo pedido.

## 3. Replanificar

En cada punto de replanificación sigue §5.9 de la plantilla. Lo que cambie del
plan va en un PR de solo documentación antes de la rebanada siguiente.

## 4. Sincronizar con el harness (`sincronizar <tag>`)

Cuando el proyecto copia `tools/architecture` de una versión nueva del harness.
Si el CHANGELOG de esa versión (o de las que salta) cambia la clave o el conteo
de las entradas, la línea base tiene que volver a llenarse. Es la única
excepción a "solo se achica", y por eso tiene su propio procedimiento:

1. Rama desde `main` y **PR propio, sin cambios de código** de producción: así
   la diferencia en la línea base solo puede venir del harness.
2. Comprueba la variable: `echo $CRAFT_BACKEND_BASELINE_INIT` tiene que dar `1`.
   Si no, detente: el usuario reabre con `CRAFT_BACKEND_BASELINE_INIT=1 claude
   --continue` (exportarla a mitad de sesión no llega al hook).
3. Guarda la línea base vigente: `git show main:tools/architecture/baseline.txt
   > /tmp/baseline-old.txt`.
4. Copia `tools/architecture` del `<tag>` (en Rust, de la plantilla del stack)
   sin tocar `baseline.txt` ni `material-exempt.txt`, y conserva lo que el proyecto adapta (anótalo en
   el PR). Vacía las entradas (deja la cabecera nueva), corre los tests de
   arquitectura y escribe una entrada por cada violación reportada.
5. Compara con el script de esta skill: `python3
   "${CLAUDE_SKILL_DIR}/scripts/compare_baseline.py" /tmp/baseline-old.txt
   tools/architecture/baseline.txt` (`${CLAUDE_SKILL_DIR}` es el directorio de
   este archivo, no el del proyecto). Agrupa en: se mantienen, cambian de clave (mapeo viejo → nuevo),
   desaparecen, bajan o suben de conteo, y nuevas.
6. **Si algo desaparece o baja (salida 1), detente y avisa.** Sin cambios de
   código, nada debería cerrarse: puede ser un cierre en falso (la versión nueva
   dejó de detectar la violación). No lo expliques tú; lo decide el usuario
   con evidencia.
7. Cada entrada nueva o que sube necesita un ID del inventario (uno existente o
   el siguiente libre de su regla) y una rebanada que la cierre. Actualiza §2,
   §5.6 y la fila de §5.10.
8. El análisis completo del script va en el cuerpo del PR. El PR, como en las
   rebanadas, solo si el usuario lo pidió.
