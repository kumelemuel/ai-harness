# ai-harness

Harness de IA reutilizable con mis convenciones de arquitectura y artesanía de
software. Los principios se escriben una sola vez, sin atarse a ninguna
herramienta, en `core/`. Los plugins de Claude Code los empaquetan como skills,
subagentes y hooks que el agente carga solo cuando los necesita.

## Estructura

```
core/principles/        Fuente de verdad, un archivo por tema (HEX, ERR, TDD, VER, WEB)
stacks/                 Traducción por stack; rust/template y astro/template son ejecutables
plugins/craft-backend/  Plugin de Claude Code: hexagonal, errores, TDD
.claude-plugin/         Este repo funciona como marketplace de plugins
```

Las skills enlazan (symlink) los documentos de `core/` en vez de copiarlos, así
que hay una sola copia de cada principio.

## Plugins

| Plugin | Contenido |
|--------|-----------|
| `craft-backend` | Skills `hexagonal-architecture`, `error-handling`, `tdd` y `rust-stack`; subagente `architecture-reviewer`; hooks `PostToolUse` y `Stop` para workspaces Rust |

## Hooks

`craft-backend` trae dos hooks que se activan solos en workspaces Rust con
`tools/architecture`:

| Hook | Cuándo | Qué hace |
|------|--------|----------|
| `PostToolUse` | Tras editar un `.rs` o `Cargo.toml` | `rustfmt` del archivo y tests de arquitectura; las violaciones vuelven al agente con su ID |
| `Stop` | Antes de que el agente termine, si hay cambios Rust | `clippy -D warnings` (solo crates de la arquitectura) + `cargo test --workspace`; si falla, no lo deja terminar (una vez por turno) |

Variables: `CRAFT_BACKEND_HOOKS=0` apaga ambos, `CRAFT_BACKEND_STOP_GATE=0` solo el
de Stop y `CRAFT_BACKEND_HOOKS_LOG=/ruta` registra cada decisión.

## Instalación

Desde GitHub:

```
/plugin marketplace add kumelemuel/ai-harness
/plugin install craft-backend@ai-harness
```

Desde un clon local, para desarrollar el harness:

```
/plugin marketplace add /ruta/a/ai-harness
/plugin install craft-backend@ai-harness
```
