# Principios

Conocimiento agnóstico de herramienta y de stack. Cada archivo cubre un solo
tema y es la **fuente de verdad**: las skills de los plugins lo enlazan, no lo
copian.

| Archivo | Prefijo | Tema |
|---------|---------|------|
| [hexagonal.md](hexagonal.md) | `HEX` | Capas, dependencias, puertos, adaptadores, responsabilidades, kernel |
| [errors.md](errors.md) | `ERR` | Árboles de errores, contratos de error, semántica de adaptadores, traducción al transporte |
| [tdd.md](tdd.md) | `TDD` | Ciclo rojo → verde → refactor y estrategia de tests por capa |
| [verification.md](verification.md) | `VER` | Enforcement automático de las reglas |
| [web.md](web.md) | `WEB` | Sitios web informativos: estático, rutas, contenido, a11y, rendimiento, CSP, privacidad, formularios, SEO, secretos |

La traducción de cada concepto a un lenguaje concreto vive en
[`stacks/`](../../stacks/README.md), no aquí.

## Formato de un principio

Cada principio tiene cuatro partes:

- **Regla** — lo que se hace. Imperativo, corto.
- **Fundamento** — por qué. Si no hay fundamento, la regla no entra.
- **Violaciones típicas** — lo que un agente de IA (o una persona) suele hacer mal.
- **Verificación** — cómo se comprueba sin depender de la revisión humana.

Cada archivo cierra con sus **antipatrones** (tabla "en vez de → usar") y sus
**decisiones abiertas**.

## Identificadores

Los IDs (`HEX-4`, `ERR-2`) son estables: se citan desde skills, subagentes,
revisiones y commits. Si un principio se elimina, su ID no se reutiliza.
