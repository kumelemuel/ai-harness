# Ejemplos canónicos (Rust)

Los archivos de la plantilla que muestran un cambio de punta a punta: el caso
de uso `create_community` del contexto `communities`. **Lee estos, en este
orden, en vez de explorar `template/`.** Abre otro archivo de la plantilla
solo si lo que necesitas no está en esta lista.

Las rutas son relativas a `references/template/`.

| Archivo | Qué ilustra |
|---------|-------------|
| `contexts/communities/domain/src/community.rs` | Agregado: campos privados, constructor, getters y su test puro (HEX-7) |
| `contexts/communities/domain/src/value_objects/community_slug.rs` | Value object que valida y su test |
| `contexts/communities/domain/src/errors.rs` | Errores de dominio planos, sin `std::error::Error` (RS-2) |
| `contexts/communities/application/src/commands/create_community/command.rs` | Comando: datos primitivos de entrada |
| `contexts/communities/application/src/commands/create_community/response.rs` | Respuesta y su `From<&Agregado>` |
| `contexts/communities/application/src/commands/create_community/error.rs` | Contrato `{Verb}{Noun}Error`: una variante por hoja y `From` por hoja (ERR-2) |
| `contexts/communities/application/src/errors.rs` | Hojas compartidas del contexto |
| `contexts/communities/application/src/commands/create_community/handler.rs` | Handler que orquesta (HEX-7) y, en el mismo archivo, un test por variante con fakes (ERR-2, TDD-2) |
| `contexts/communities/application/src/ports/inbound/create_community.rs` | Puerto entrante `{Verb}{Noun}UseCase` con `BoxFuture` (RS-3) |
| `contexts/communities/application/src/ports/outbound/community_repository.rs` | Puerto saliente sin errores declarados (`Result<T, Fault>`) |
| `contexts/communities/application/src/ports/outbound/moderation_gateway.rs` | Puerto saliente con error declarado y su semántica (ERR-3) |
| `contexts/communities/application/src/ports/outbound/dtos/moderation_payloads.rs` | Payloads del puerto: request, veredicto y error (HEX-6) |
| `contexts/communities/application/src/testing.rs` | Dobles `Fake…Adapter` que corren la batería de su puerto (HEX-5, TDD-2) |
| `contexts/communities/application/src/testing/contracts/community_repository.rs` | Batería de contrato `{puerto}_contract!` |
| `contexts/communities/application/src/testing/contracts/moderation_gateway.rs` | Batería con un escenario obligatorio por variante (ERR-3) |
| `contexts/communities/infrastructure/src/in_memory_community.rs` | Adaptador `{Tecnología}{Concepto}Adapter` que corre la batería |
| `contexts/communities/api/src/create_community.rs` | Endpoint: `HttpRequest`/`HttpResponse`, `match` exhaustivo y un helper por hoja (ERR-4) |
| `contexts/communities/api/src/codes.rs` | Códigos estables de error para clientes |
| `contexts/communities/api/src/lib.rs` | `{Ctx}Api` con contratos `Arc<dyn …UseCase>` y sus rutas |
| `contexts/communities/api/tests/create_community.rs` | Test del endpoint con `oneshot` y un caso de uso stub, uno por variante |
| `edge/http/src/problem.rs` | Problem Details compartido: constructores por tipo de problema |
| `apps/server/src/main.rs` | Composition root: el único que conoce implementaciones (HEX-2) |

La plantilla no tiene una consulta (`application/src/queries/`): una consulta
sigue la forma del comando, con `query.rs` en lugar de `command.rs`.
