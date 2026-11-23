# Adopción del harness craft-backend

<!--
Plantilla de la skill `craft-backend:adoption`. Copiar a `docs/harness-adoption.md`
del proyecto y reemplazar todo lo que está entre `<…>`. Este archivo guarda el
estado de la adopción entre sesiones: cada sesión lo lee entero antes de empezar.
-->

Este archivo guarda el estado entre sesiones de la adopción de los principios
craft-backend (HEX, ERR, TDD, VER y los del stack) en este repositorio.

| Dato | Valor |
|---|---|
| Fecha de la auditoría | <AAAA-MM-DD> |
| Plugin | `craft-backend@ai-harness` <versión> |
| Commit auditado | `<sha>` (rama `<rama>`) |
| Revisor | `craft-backend:architecture-reviewer`, auditoría completa en solo lectura |
| Hallazgos | <N>: <a> de severidad alta, <m> media, <b> baja |

## Cómo mantener este archivo

- Al resolver un hallazgo, cambiar su **Estado** de `abierto` a
  `resuelto (<commit>)`. Si se descarta, poner `descartado` y el motivo.
- Los IDs de hallazgo (`HEX-1.3`, `ERR-4.2`…) son estables: no se renumeran. Un
  hallazgo nuevo recibe el siguiente número libre de su regla.
- Después de cada punto de replanificación (§5.9), volver a auditar con
  `architecture-reviewer` y actualizar la cabecera (fecha, commit, conteos).
- <Red de seguridad: dónde viven los tests de caracterización, si los hay.>

## Prioridades inmediatas

1. **<ID>:** <lo que no puede esperar a su rebanada, como un secreto versionado>.

Ya resueltas: <IDs y commit>.

## 1. Resumen

| Regla | Alta | Media | Baja | Total |
|---|---|---|---|---|
| HEX-1 Dependencias hacia adentro | | | | |
| <una fila por regla con hallazgos> | | | | |
| **Total** | | | | |

Cada fila del inventario cuenta como un hallazgo. Cuando una misma violación se
repite igual en varios archivos, la fila lista todas las ubicaciones.

## 2. Inventario

Una tabla por skill. **Corrección** dice cómo queda según el principio, no cómo
llegar.

### HEX (hexagonal-architecture) y VER

| ID | Regla | archivo:línea | Violación | Sev. | Corrección | Estado |
|---|---|---|---|---|---|---|
| HEX-1.1 | HEX-1 Dependencias | <ruta:línea> | <qué viola> | alta | <cómo queda> | abierto |

### ERR (error-handling)

| ID | Regla | archivo:línea | Violación | Sev. | Corrección | Estado |
|---|---|---|---|---|---|---|

### TDD (tdd)

| ID | Regla | archivo:línea | Violación | Sev. | Corrección | Estado |
|---|---|---|---|---|---|---|

### <Stack> (<skill del stack>)

| ID | Regla | archivo:línea | Violación | Sev. | Corrección | Estado |
|---|---|---|---|---|---|---|

## 3. Reglas que se cumplen

Sin ninguna violación:
- **<ID>:** <por qué se cumple>.

Partes que sí se cumplen dentro de reglas que tienen hallazgos:
- **<ID> (<aspecto>):** <qué se cumple>.

## 4. Dudas y hallazgos plausibles sin confirmar

No cuentan en el inventario. Si se confirman, pasan a él con el siguiente ID
libre de su regla.

- **<Tema> (<ID de regla>).** <Qué se vio y qué falta para confirmarlo.>

## 5. Plan de rebanadas

Las rebanadas se citan como **R1…Rn**. Cada una es un PR que sale de `main`,
sin PRs apilados, y tiene que estar verde en CI. En ese mismo PR se actualiza el
inventario (`resuelto (<commit>)`), y el avance se anota en §5.8.

Datos verificados que condicionan el plan:
- `tools/architecture/baseline.txt` tiene <n> entradas. Coincide con la suma de §5.6.
- <Qué hacen y qué no hacen hoy los tests de arquitectura sobre este repo.>

### 5.1 Reglas transversales

- **Trinquete.** `baseline.txt` solo se achica, salvo en una sincronización con el
  harness (§5.10). Si un archivo que se mueve o
  renombra figura en la línea base, la violación se corrige en el lugar y la
  entrada se borra antes de mover. Antes de abrir cada PR: <comando de los
  tests de arquitectura del stack; en Rust, `cargo test -p architecture`>.
- **Red de seguridad.** <Qué tests no se modifican, salvo los cambios de
  contrato de §5.5.>
- **TDD.** Rojo primero también en un refactor de arquitectura: se borra o
  baja la entrada de la línea base, se confirma que el test falla con esa
  violación y se corrige hasta verde.
- <Verificación manual mientras haya algo que los tests no ven.>

### 5.2 <Decisión de orden: qué se corrige en el lugar y qué espera a la separación>

<Recomendación y por qué. Por defecto: corregir en el lugar antes de mover.>

### 5.3 Secuencia

Tamaños: XS (<50 líneas), S (<200), M (<500), L (>500 o muchos archivos).

#### Fase <n>: <nombre>

**R<n>. <Título>.**
- **IDs:** <hallazgos que cierra>.
- **Toca:** <archivos>.
- **Línea base:** <−k (qué entradas)> o sin cambios.
- **Rojo:** <tests que fallan antes del cambio>.
- **Contrato externo** (HTTP u otro transporte): <sin cambios, o qué cambia (§5.5)>.
- **Riesgo:** <qué puede romperse>.
- **Tamaño:** <XS | S | M | L>.
- **Resultado:** <al terminar: commits, rojo visto, lo que señaló el revisor
  y quedó para otra rebanada>.

> **Punto de replanificación P<n> (después de R<n>).** Ver §5.9.

### 5.4 Dependencias y paralelismo

```
R1 ─┬─ R2
    └─ R3 ─ …
```

### 5.5 Cambios del contrato externo (todos explícitos)

| Rebanada | Cambio | Tests de caracterización que cambian |
|---|---|---|

### 5.6 Cierre de cada test de arquitectura

| Test | Entradas hoy | Queda en cero en |
|---|---|---|

Los hallazgos que no son de código no tienen rebanada: <IDs y por qué>.

### 5.7 Decisiones del usuario

Decididas (<fecha>):
- **D<n>. <Tema> (<rebanadas>): decidido, <qué>.** <Detalle.>

Abiertas:
- **D<n>. <Tema> (<rebanadas>).** <Opciones a, b, c y cuál propone el plan.>

### 5.8 Registro de rebanadas

Una fila por rebanada. La fila y el `resuelto (…)` del inventario se escriben en
el mismo PR, con el número de PR; el commit de `main` (que cambia con un squash)
se completa en el PR de documentación siguiente. **Tokens** es el consumo de la sesión que hizo
la rebanada (lo que informa `/cost` al cerrarla). **Revisor** dice si se corrió
`craft-backend:architecture-reviewer` y, si no, por qué.

| Rebanada | PR | Commit | Toolchain | Tokens | Revisor | Notas |
|---|---|---|---|---|---|---|

### 5.9 Puntos de replanificación

En cada uno, antes de abrir la rebanada siguiente:

1. Auditoría completa con `architecture-reviewer` y actualización de la cabecera.
2. Los tests de arquitectura del stack, y comparar las entradas de `baseline.txt` con
   §5.6 menos lo ya cerrado. Si no coinciden, explicar la diferencia.
3. Revisar las decisiones abiertas de §5.7 que bloquean lo que sigue.
4. Comparar tamaño y tokens reales de §5.8 con lo estimado, y partir lo que
   viene si algo quedó más grande de lo previsto.
5. Reescribir aquí lo que cambie del plan, en un PR de solo documentación.

### 5.10 Sincronizaciones con el harness

Una por PR propio, sin cambios de código (`/craft-backend:adoption sincronizar
<tag>`). El análisis completo de `compare_baseline.py` va en el PR; aquí, el
resumen.

| Fecha | Tag | PR | Entradas antes → después | Se mantienen | Cambian de clave | Nuevas (IDs) |
|---|---|---|---|---|---|---|

## 6. Fricciones con el harness

Lo que el harness no cubrió o cubrió mal durante la adopción. Cada fricción se
reporta al harness. Al resolverse, se anota la versión que la cierra.

| Fecha | Rebanada | Fricción | Efecto en la adopción | Estado en el harness |
|---|---|---|---|---|

## Método

- <Qué se leyó y cómo se verificó cada `archivo:línea`.>
- <Qué se ejecutó y qué no.>
