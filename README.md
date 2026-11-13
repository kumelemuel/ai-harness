# ai-harness

Harness de IA reutilizable con mis convenciones de arquitectura y artesanía de
software. Los principios se escriben una sola vez, sin atarse a ninguna
herramienta, en `core/`. Los plugins de Claude Code los empaquetan como skills,
subagentes y hooks que el agente carga solo cuando los necesita.

## Estructura

```
core/principles/        Fuente de verdad, un archivo por tema (HEX, ERR, TDD, VER, WEB)
core/verification.md    Inventario: cómo se verifica cada principio y decisión de stack
stacks/                 Traducción por stack; rust/template y astro/template son ejecutables
templates/web/          Proceso comercial: descubrimiento, spec, cotización, entrega
plugins/craft-backend/  Plugin de Claude Code: hexagonal, errores, TDD, Rust
plugins/craft-web/      Plugin de Claude Code: principios WEB, Astro, proceso comercial
scripts/                Verificador de consistencia, runner de katas y sus tests
evals/                  Katas: corrección y consumo con sesiones reales
CLAUDE.md               Cómo se hace un cambio en este repo
.claude-plugin/         Este repo funciona como marketplace de plugins
```

Las skills enlazan (symlink) los documentos de `core/` en vez de copiarlos, así
que hay una sola copia de cada principio.

## Plugins

| Plugin | Contenido |
|--------|-----------|
| `craft-backend` | Skills `hexagonal-architecture`, `error-handling`, `tdd` y `rust-stack`; subagente `architecture-reviewer`; hooks `PostToolUse` y `Stop` para workspaces Rust |
| `craft-web` | Skills `web-principles`, `astro-stack` y `client-delivery`; subagente `site-auditor`; hooks `PostToolUse` y `Stop` para sitios Astro con `src/routes.ts` |

## Hooks

`craft-backend` trae dos hooks que se activan solos en workspaces Rust con
`tools/architecture`:

| Hook | Cuándo | Qué hace |
|------|--------|----------|
| `PostToolUse` | Tras editar un `.rs` o `Cargo.toml` | `rustfmt` del archivo y tests de arquitectura; las violaciones vuelven al agente con su ID |
| `Stop` | Antes de que el agente termine, si hay cambios Rust | `clippy -D warnings` (solo crates de la arquitectura) + `cargo test --workspace`; si falla, no lo deja terminar (una vez por turno) |

Variables: `CRAFT_BACKEND_HOOKS=0` apaga ambos, `CRAFT_BACKEND_STOP_GATE=0` solo el
de Stop y `CRAFT_BACKEND_HOOKS_LOG=/ruta` registra cada decisión.

Los hooks de ambos plugins requieren `python3` 3.11 o superior en el `PATH`
(`craft-backend` lee los `Cargo.toml` con `tomllib`). Con una versión menor, cada
hook lo avisa por stderr y sale con 0: un entorno incompleto no bloquea al
agente, igual que cuando falta `cargo`, `rustfmt` o `node`. En ese caso no se
verifica nada.

## Verificación del harness

El harness se somete a VER-1 como cualquier proyecto. `scripts/check_harness.py`
(Python 3.11+, solo biblioteca estándar) falla con una línea por problema:

```
python3 scripts/check_harness.py                       # sobre el repo
python3 scripts/check_harness.py --base main           # además, lo que entra a main
python3 -m unittest discover -s scripts/tests          # tests del verificador
```

| Código | Qué detecta |
|--------|-------------|
| `ID-UNDEFINED` | Un ID citado (`HEX-n`, `ERR-n`, `TDD-n`, `VER-n`, `WEB-n`, `RS-n`, `AS-n`) que no está definido en su archivo fuente |
| `ID-DUPLICATE` | Un ID definido dos veces |
| `SYMLINK` | Un symlink que no resuelve |
| `MD-LINK` | Un enlace relativo de Markdown que no resuelve |
| `FRONTMATTER` | Un `SKILL.md` o subagente sin `name` o `description`, o con un `name` que no coincide con su carpeta o archivo |
| `STACK-PATH` | Una ruta entre comillas invertidas en la descripción de una skill de stack que no existe en la plantilla |
| `CLAUDE-MD-SIZE` | El `CLAUDE.md` de la raíz supera 800 palabras: es el procedimiento de trabajo, no un manual |
| `STACK-STATUS` | Una versión de stack distinta en `stacks/README.md` y en su guía |
| `VOLATILE-COUNT` | Un conteo de tests en la documentación (fuera del CHANGELOG) |
| `MISSING-TEST` | Un test citado entre comillas invertidas en una sección "Verificación", en una tabla de Enforcement o en una fila `test` del inventario que no existe en la plantilla (o una fila `test` que no cita ninguno): una función con prefijo de principio (`hex_4_…`, admite `*`), con o sin archivo (`tests/naming.rs::hex_4_*`), o un archivo de test (`routes.test.ts`) |
| `VERIF-MISSING-ID` | Un ID definido en `core/principles/` o en una guía de stack sin fila en [`core/verification.md`](core/verification.md) |
| `VERIF-UNKNOWN-ID` | Una fila del inventario con un ID que no existe |
| `VERIF-BAD-MECHANISM` | Un mecanismo fuera de la lista cerrada (`test`, `herramienta`, `revisor:<agente>`, `pendiente`), o una `herramienta` o un `pendiente` sin detalle |
| `VERIF-REVIEWER` | Un `revisor:<agente>` que no existe o cuyo archivo no menciona el ID |
| `VERSION-NOT-BUMPED` | Con `--base`: cambió un archivo que llega a un plugin y su versión no subió |
| `CHANGELOG-UNRELEASED` | Con `--base`: el CHANGELOG tiene una sección "Sin publicar" |

En las descripciones de las skills de stack (las que tienen `references/template`),
toda ruta va entre comillas invertidas para que el verificador la compruebe. La
descripción decide cuándo se carga la skill: una ruta que ya no existe cambia el
comportamiento, no solo la documentación.

La documentación no lleva números volátiles (conteos de tests): dice "todos en
verde" y el CI lo demuestra. Solo el CHANGELOG conserva conteos, porque registra
un momento. `VOLATILE-COUNT` lo hace cumplir.

Un test citado como garantía tiene que existir. `MISSING-TEST` revisa los
párrafos `**Verificación…**`, las secciones con encabezado "Verificación" o
"Enforcement" y las tablas con una columna así, fuera de los bloques de código y
del CHANGELOG. Un documento de `stacks/<x>/` se compara con `stacks/<x>/template`;
los demás, con todas las plantillas. Solo cuentan como test las funciones de un
directorio `tests/` y las que llevan `#[test]`: un helper con el mismo prefijo
no sostiene la promesa.

El CI ([`.github/workflows/harness.yml`](.github/workflows/harness.yml)) corre
el verificador y sus tests (con `--base` en los pull requests hacia `main` y en
los push a `main`); en la plantilla Rust, `cargo fmt --check`,
`clippy -D warnings` y `cargo test --workspace`, y en la plantilla Astro, lint,
`astro check` y los tests unitarios. El job `astro-e2e` hace el build de la
plantilla Astro y corre los e2e con Playwright contra `wrangler dev` y los
presupuestos de Lighthouse CI (WEB-5). Corre cuando un pull request o un push
cambia `stacks/astro/`, cada semana (lunes) y a mano (`workflow_dispatch`); la
ejecución semanal solo corre ese job.

## Publicar una versión

El marketplace apunta a `main`, y Claude Code detecta una actualización solo
cuando cambia la `version` del `plugin.json`. Si el contenido cambia y la
versión no, quien ya tenía esa versión no recibe el cambio, y quien instala
recibe contenido nuevo bajo un número viejo. Por eso:

- **Cada merge a `main` que cambie el contenido de un plugin sube su versión.**
  Una corrección es un patch; una regla o una skill nueva, un minor.
- El contenido de un plugin es su directorio y lo que alcanza por symlinks:
  `core/principles/tdd.md` es contenido de `craft-backend`, y
  `stacks/astro/template/src` de `craft-web`.
- "Sin publicar" solo existe en ramas. Antes del merge, esas entradas pasan a
  una sección `## X.Y.Z — fecha` del CHANGELOG que nombra la versión de cada
  plugin.

`check_harness.py --base main` lo comprueba en la rama
(`VERSION-NOT-BUMPED` y `CHANGELOG-UNRELEASED`), y el CI lo exige en cada pull
request hacia `main`.

## Instalación

Instala cada plugin con alcance de proyecto, en el repo donde aplica:
`craft-backend` en los backends y `craft-web` en los sitios.

```
/plugin marketplace add kumelemuel/ai-harness

# En un backend
claude plugin install craft-backend@ai-harness --scope project

# En un sitio Astro
claude plugin install craft-web@ai-harness --scope project
```

Por qué: los hooks se activan solos al detectar el tipo de proyecto, pero las
skills se cargan por su descripción. Las descripciones nombran el tipo de
proyecto donde aplican, aunque eso solo reduce el riesgo. Con ambos plugins a
nivel de usuario, el agente ve las siete skills en todos los repos y puede
cargar, por ejemplo, `tdd` (handlers con fakes de puertos) en un sitio Astro.
El alcance de proyecto queda en `.claude/settings.json`, se versiona con el
repo y le da a todo el equipo el mismo harness.

## Desarrollar el harness

Agrega el clon como marketplace y habilita los plugins en el proyecto de
prueba, **sin `claude plugin install`**:

```
claude plugin marketplace add /ruta/a/ai-harness
```

```json
// .claude/settings.json del proyecto de prueba
{
  "enabledPlugins": {
    "craft-backend@ai-harness": true
  }
}
```

Con un marketplace agregado desde una ruta local, Claude Code carga los plugins
en su lugar, desde el clon. Una edición se ve en la sesión siguiente o con
`/reload-plugins`, sin reinstalar y sin subir la versión.

No uses `claude plugin install` desde el clon: la sesión tampoco lee esa copia,
pero la instalación copia el plugin a la caché con todo lo que hay en disco,
incluidos los artefactos sin versionar. `craft-backend` arrastra el `target/`
de la plantilla Rust (cerca de 1 GB). Verificado con Claude Code 2.1.316.
