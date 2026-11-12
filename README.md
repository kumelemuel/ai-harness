# ai-harness

Harness de IA reutilizable con mis convenciones de arquitectura y artesanía de
software. Los principios se escriben una sola vez, sin atarse a ninguna
herramienta, en `core/`. Los plugins de Claude Code los empaquetan como skills,
subagentes y hooks que el agente carga solo cuando los necesita.

## Estructura

```
core/principles/        Fuente de verdad, un archivo por tema (HEX, ERR, TDD, VER)
stacks/                 Traducción por lenguaje; stacks/rust/template es un workspace ejecutable
plugins/craft-backend/  Plugin de Claude Code: hexagonal, errores, TDD
.claude-plugin/         Este repo funciona como marketplace de plugins
```

Las skills enlazan (symlink) los documentos de `core/` en vez de copiarlos, así
que hay una sola copia de cada principio.

## Plugins

| Plugin | Contenido |
|--------|-----------|
| `craft-backend` | Skills `hexagonal-architecture`, `error-handling`, `tdd` y `rust-stack`; subagente `architecture-reviewer` |

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
