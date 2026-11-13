# Mapa del proyecto

Workspace Cargo hexagonal: un crate por capa y por contexto. Las reglas están
en las skills de `craft-backend`; esto es solo dónde vive cada cosa.

## Capas

| Capa | Dónde |
|------|-------|
| Kernel compartido (`Failure`, `Fault`, `BoxFuture`, ids) | `kernel/src/` |
| Dominio de un contexto | `contexts/communities/domain/src/` |
| Casos de uso, puertos y dobles | `contexts/communities/application/src/` |
| Adaptadores salientes | `contexts/communities/infrastructure/src/` |
| Adaptadores HTTP del contexto | `contexts/communities/api/src/` |
| Borde HTTP compartido (Problem Details, identidad) | `edge/http/src/` |
| Composition root | `apps/server/src/main.rs` |
| Tests de arquitectura | `tools/architecture/tests/` |

## Recorrido de un caso de uso (`create_community`)

| Pieza | Archivo |
|-------|---------|
| Comando y respuesta | `contexts/communities/application/src/commands/create_community/command.rs`, `response.rs` |
| Contrato de error | `contexts/communities/application/src/commands/create_community/error.rs` |
| Hojas de error compartidas | `contexts/communities/application/src/errors.rs` |
| Handler y sus tests (mismo archivo) | `contexts/communities/application/src/commands/create_community/handler.rs` |
| Puerto entrante | `contexts/communities/application/src/ports/inbound/create_community.rs` |
| Puertos salientes | `contexts/communities/application/src/ports/outbound/` |
| Dobles y baterías de contrato | `contexts/communities/application/src/testing.rs`, `contexts/communities/application/src/testing/contracts/` |
| Adaptador | `contexts/communities/infrastructure/src/in_memory_community.rs` |
| Endpoint, rutas y códigos | `contexts/communities/api/src/create_community.rs`, `contexts/communities/api/src/lib.rs`, `contexts/communities/api/src/codes.rs` |
| Test del endpoint | `contexts/communities/api/tests/create_community.rs` |
| Wiring | `apps/server/src/main.rs` |

Una consulta va en `application/src/queries/{caso}/`, con la misma forma.

## Comandos

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p architecture   # solo las reglas de arquitectura
```
