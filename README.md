# ai-harness

Harness de IA reutilizable con mis convenciones de arquitectura y artesanía de
software. Los principios se escriben una sola vez, sin atarse a ninguna
herramienta, en `core/`. Los plugins de Claude Code los empaquetan como skills,
subagentes y hooks que el agente carga solo cuando los necesita.

## Estructura

```
core/principles/        Fuente de verdad, un archivo por tema (HEX, ERR, TDD, VER, WEB)
stacks/                 Traducción por stack; rust/template y astro/template son ejecutables
templates/web/          Proceso comercial: descubrimiento, spec, cotización, entrega
plugins/craft-backend/  Plugin de Claude Code: hexagonal, errores, TDD, Rust
plugins/craft-web/      Plugin de Claude Code: principios WEB, Astro, proceso comercial
scripts/                Verificador de consistencia del harness y sus tests
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

## Verificación del harness

El harness se somete a VER-1 como cualquier proyecto. `scripts/check_harness.py`
(Python 3.11+, solo biblioteca estándar) falla con una línea por problema:

```
python3 scripts/check_harness.py                       # sobre el repo
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
| `STACK-STATUS` | Una versión de stack distinta en `stacks/README.md` y en su guía |
| `VOLATILE-COUNT` | Un conteo de tests en la documentación (fuera del CHANGELOG) |

En las descripciones de las skills de stack (las que tienen `references/template`),
toda ruta va entre comillas invertidas para que el verificador la compruebe. La
descripción decide cuándo se carga la skill: una ruta que ya no existe cambia el
comportamiento, no solo la documentación.

La documentación no lleva números volátiles (conteos de tests): dice "todos en
verde" y el CI lo demuestra. Solo el CHANGELOG conserva conteos, porque registra
un momento. `VOLATILE-COUNT` lo hace cumplir.

El CI ([`.github/workflows/harness.yml`](.github/workflows/harness.yml)) corre
el verificador y sus tests; en la plantilla Rust, `cargo fmt --check`,
`clippy -D warnings` y `cargo test --workspace`, y en la plantilla Astro, lint,
`astro check` y los tests unitarios. Los e2e y Lighthouse de Astro corren en el
CI propio de la plantilla.

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

Desde un clon local, para desarrollar el harness:

```
/plugin marketplace add /ruta/a/ai-harness
claude plugin install craft-backend@ai-harness --scope project
```
