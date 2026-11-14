# ai-harness: cómo se trabaja en este repo

Los principios viven en [`core/principles/`](core/principles/README.md) y el
inventario de cómo se verifica cada uno, en
[`core/verification.md`](core/verification.md). Este archivo no los repite:
dice cómo se hace un cambio. La estructura y la publicación están en el
[`README.md`](README.md).

## Procedimiento de un cambio

1. **Rama.** Nunca se trabaja en `main`. Commit en la rama; merge y push solo
   cuando el usuario lo pide.
2. **Rojo primero.** Toda regla nueva del verificador, de un hook o de un test
   de arquitectura empieza con su test fallando: un fixture en
   `scripts/tests/fixtures/` o un test en la plantilla.
3. **Inyectar y revertir.** Con la regla en verde, inyecta cada violación en el
   repo real (o en la plantilla), confirma que se detecta con su código o ID, y
   revierte. Anota cada inyección en el mensaje de commit.
4. **Todo en verde:**
   ```
   python3 -B -m unittest discover -s scripts/tests
   python3 -B scripts/check_harness.py
   python3 -B scripts/check_harness.py --base main   # antes del merge
   ```
   Si tocas una plantilla, también su CI local (`cargo fmt --check`, `clippy`,
   `cargo test --workspace` con `ARCH_REQUIRE_MATERIAL=1`; o lint, `astro
   check` y `npm test` con Node 24).
5. **Versión (semver).** Cada merge a `main` que cambie el contenido de un plugin
   sube su `version` en `plugin.json`: un patch para una corrección, un minor
   para una regla o una skill nueva. El contenido incluye lo que alcanza por
   symlink (`core/principles/`, `stacks/*/`).
6. **CHANGELOG** en el formato de abajo.

## Katas

`evals/` mide corrección y consumo con sesiones reales. Ver
[`evals/README.md`](evals/README.md). Córrelas cuando cambian skills, hooks,
subagentes o principios, y anota el resultado en el CHANGELOG. No corren en CI.

- Fija siempre el modelo (`--model`).
- **Una variable por configuración.** Si cambian dos cosas (un archivo y el
  texto de una skill, o el harness y el modelo), el efecto no se puede atribuir.
- **El control corre intercalado, en la misma ventana de tiempo**
  (`--subagent-model A --subagent-model B`, o las dos configuraciones en
  paralelo). No se compara contra una línea base de otro día: el control del
  renombre dio $1.33–1.63 contra los $1.35–1.37 de la línea base.
- Con 3 corridas, un efecto solo se da por establecido si los rangos de las dos
  configuraciones no se solapan.
- Un subagente se mide como subagente (`subagent` en `kata.toml`), no con
  `--agent`, que no precarga sus `skills:`.
- **Fijar `model:` en un subagente** exige que el modelo más económico detecte
  todas las violaciones plantadas en las 3 corridas, y que sus falsos positivos
  y hallazgos con ID equivocado (clasificados a mano) no superen los del modelo
  de referencia.
- Los plugins cargan en su lugar, desde el clon: **no edites `plugins/` ni lo
  que enlazan mientras corre una kata**, o la medición mezcla dos versiones.
- Una corrida cortada por la API queda inválida; vuelve a correr con la misma
  etiqueta para repetirla.

## CHANGELOG

Para agregar una entrada, lee solo el comienzo del archivo (la sección más
reciente), no el archivo completo.

Desde 0.12.0, cada versión tiene:

```
## X.Y.Z — AAAA-MM-DD

Plugins: `craft-backend` A.B.C, `craft-web` D.E.F.

### <Agregado | Cambiado | Corregido>
- **<Qué cambió>** (IDs que toca): por qué, en una o dos líneas.

### Filas que cambiaron de mecanismo
- <ID> (<aspecto>): de <mecanismo> a <mecanismo>.

### Katas
- <kata>: aceptación, costo y turnos (mediana) contra la referencia.
```

Las secciones sin contenido se omiten. La narración de cada verificación
(inyecciones, comandos, salidas) va en el cuerpo del mensaje de commit, no en el
CHANGELOG. Las entradas anteriores a 0.12.0 no se reescriben.

En una rama, las entradas nuevas van bajo `## Sin publicar`. Antes del merge
pasan a su versión (`CHANGELOG-UNRELEASED` lo exige).

## Lo no previsto

Si un encargo no cubre una decisión (una premisa falsa, un efecto en otro
plugin, un costo mayor al estimado), **repórtala con una recomendación en vez
de improvisarla**. Lo mismo si el usuario pidió detenerse en un punto: se espera
su confirmación.
