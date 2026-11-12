# Inventario de verificación

> **Relacionados:** [VER-1](principles/verification.md) (lo que no se verifica se erosiona).

Cómo se comprueba cada principio (`core/principles/`) y cada decisión de stack
(`stacks/*/README.md`). Es un documento vivo: lo verifica
[`scripts/check_harness.py`](../scripts/check_harness.py), así que un ID sin fila, un test
citado que no existe o un revisor que no sabe que le toca fallan el CI.

Un ID puede tener varias filas, una por aspecto, cuando cada aspecto se verifica
distinto.

## Mecanismos

La lista es **cerrada**. No existe "sin revisor": lo que no se automatiza, se asigna.

| Mecanismo | Significa | Lo comprueba el verificador |
|-----------|-----------|-----------------------------|
| `test` | Un test lo garantiza. El detalle lo cita | `MISSING-TEST`: el test existe en la plantilla |
| `herramienta` | Compilador, linter, Lighthouse, el CI u otra. El detalle la nombra | `VERIF-BAD-MECHANISM`: el detalle no está vacío |
| `revisor:<agente>` | No es automatizable de forma fiable: lo revisa un subagente | `VERIF-REVIEWER`: el agente existe y su archivo menciona el ID |
| `pendiente` | Es automatizable, pero todavía no tiene test. El detalle explica por qué. Es válido, pero visible | `VERIF-BAD-MECHANISM`: el detalle no está vacío |

El CHANGELOG no copia este inventario: cada versión registra solo las filas que
cambiaron de mecanismo.

## Inventario

| ID | Aspecto | Mecanismo | Detalle |
|----|---------|-----------|---------|
| HEX-1 | Dependencias hacia adentro, `api` solo ve su `application` y `edge`, sin cruces entre contextos | `test` | `tests/dependencies.rs::hex_1_dependencies_point_inward` |
| HEX-2 | Núcleo solo con el estándar y el kernel | `test` | `tests/dependencies.rs::hex_2_core_depends_only_on_std_and_the_kernel` |
| HEX-2 | Frameworks de servidor solo en el borde | `test` | `tests/dependencies.rs::hex_2_server_frameworks_only_at_the_edge` |
| HEX-3 | El dominio no usa las primitivas de E/S del kernel | `test` | `tests/ports.rs::hex_3_domain_does_not_use_kernel_io_primitives` |
| HEX-3 | El dominio no declara asincronía propia (`async fn`, `impl Future`) | `pendiente` | El test se acotó a las primitivas del kernel. Detectar la asincronía propia es automatizable con `syn`, pero falta definir qué cuenta como frontera técnica |
| HEX-4 | Un `UseCase` de un solo `execute` | `test` | `tests/naming.rs::hex_4_inbound_ports_are_single_method_use_cases` |
| HEX-4 | Las implementaciones se llaman `*Handler` | `test` | `tests/naming.rs::hex_4_use_case_implementations_are_handlers` |
| HEX-4 | `api` depende del `UseCase`, nunca del `Handler` | `test` | `tests/naming.rs::hex_4_api_depends_on_use_cases_never_on_handlers` |
| HEX-4 | El contrato no expone tipos de `domain` | `test` | `tests/naming.rs::hex_4_use_case_contracts_do_not_expose_domain_types` |
| HEX-5 | Puertos salientes con sufijo de rol, nunca `Port` | `test` | `tests/naming.rs::hex_5_outbound_ports_are_named_by_role` |
| HEX-5 | Adaptadores `{Tecnología}{Concepto}Adapter` en infraestructura, sin prefijos de doble | `test` | `tests/naming.rs::hex_5_adapters_are_named_by_technology_and_concept` |
| HEX-6 | DTOs de puerto saliente sin `Command`, `Query`, `Response` ni `Result` | `test` | `tests/naming.rs::hex_6_outbound_dtos_do_not_use_reserved_vocabulary` |
| HEX-6 | Tipos de transporte con el nombre del transporte | `test` | `tests/naming.rs::hex_6_transport_types_carry_the_transport_name` |
| HEX-7 | Lógica de negocio fuera del dominio | `revisor:architecture-reviewer` | Prioridad 1 del revisor: `if` de negocio, cálculos y transiciones en handlers o adaptadores |
| HEX-8 | El kernel no tiene dependencias | `test` | `tests/kernel.rs::hex_8_kernel_depends_only_on_std` |
| HEX-8 | Traits del kernel: marcadores o capacidades de la lista cerrada | `test` | `tests/kernel.rs::hex_8_kernel_traits_are_model_markers_or_listed_capabilities` |
| ERR-1 | Los errores de `domain` no implementan `std::error::Error` | `test` | `tests/errors.rs::err_1_domain_errors_do_not_implement_std_error` |
| ERR-1 | Los errores de `application` (hojas, contratos, puertos) tampoco | `test` | `tests/errors.rs::err_1_application_errors_do_not_implement_std_error` |
| ERR-1 | Excepciones para fallos de negocio, conversiones amplias entre árboles | `revisor:architecture-reviewer` | Prioridad 3 del revisor |
| ERR-2 | Exhaustividad del contrato en el borde | `herramienta` | Compilador: `match` exhaustivo sobre `{Verb}{Noun}Error` en `api` |
| ERR-2 | Un test por variante admitida del contrato | `revisor:architecture-reviewer` | Prioridad 3 del revisor: por cada variante de `{Verb}{Noun}Error`, un test del handler que la produce y uno del endpoint que la traduce |
| ERR-3 | Un caso por resultado en la batería de contrato, uno por variante declarada | `herramienta` | Compilador: `scenario_of` hace un `match` exhaustivo sobre `ModerationError`, y `moderation_gateway_contract!` exige un campo por caso. `cargo test` corre la batería contra el doble y cada adaptador. Sus casos los genera una macro: el verificador no puede citarlos |
| ERR-3 | Semántica: rechazo como éxito, sin errores pensados para el transporte | `revisor:architecture-reviewer` | Prioridad 2 del revisor |
| ERR-4 | El núcleo no escribe en la salida | `test` | `tests/output.rs::err_4_core_does_not_write_to_output` |
| ERR-4 | El núcleo no importa transporte ni logging | `test` | `tests/dependencies.rs::hex_2_core_depends_only_on_std_and_the_kernel` |
| ERR-4 | Sin datos sensibles en errores, mensajes ni registros | `revisor:architecture-reviewer` | Prioridad 4 del revisor |
| TDD-1 | Test antes o junto a la producción (backend) | `revisor:architecture-reviewer` | Diff separado en producción y test, y orden de los commits |
| TDD-1 | Test antes o junto a la producción (sitios) | `revisor:site-auditor` | Paso 8 del auditor |
| TDD-2 | Los tests del núcleo no levantan runtime ni framework | `test` | `tests/dependencies.rs::hex_2_core_depends_only_on_std_and_the_kernel` (lee `[dev-dependencies]`) |
| TDD-2 | Dobles con prefijo `Fake`, `Fixed`, `Stub` o `Spy` | `test` | `tests/naming.rs::hex_5_adapters_are_named_by_technology_and_concept` |
| TDD-2 | Doble y adaptadores cumplen el mismo contrato | `herramienta` | `cargo test`: las baterías `{puerto}_contract!` corren contra el doble y cada adaptador. Sus casos los genera una macro: el verificador no puede citarlos |
| TDD-2 | Forma de los tests: sin contenedor, fakes en vez de mocks, reglas no probadas por el endpoint | `revisor:architecture-reviewer` | Prioridad 5 del revisor |
| VER-1 | Los tests de arquitectura corren en CI y no pasan en vacío | `herramienta` | `.github/workflows/harness.yml`, job `rust`, con `ARCH_REQUIRE_MATERIAL=1` |
| VER-1 | Lo citado existe y cada regla tiene mecanismo | `herramienta` | `scripts/check_harness.py` (`MISSING-TEST` y `VERIF-*`), job `consistency` |
| RS-1 | Un crate por capa; Cargo hace cumplir las dependencias | `test` | `tests/dependencies.rs::hex_1_dependencies_point_inward` |
| RS-2 | `unwrap` prohibido | `herramienta` | Clippy, `unwrap_used = "deny"` en `[workspace.lints]` |
| RS-2 | `panic!` y `expect` solo para invariantes rotos | `revisor:architecture-reviewer` | Prioridad 6 del revisor |
| RS-3 | Puertos async con `BoxFuture`, sin `async-trait` en el núcleo | `test` | `tests/dependencies.rs::hex_2_core_depends_only_on_std_and_the_kernel` |
| RS-4 | Tests de arquitectura con `toml` y `syn` | `test` | `tests/naming.rs`, `tests/dependencies.rs`, `tests/errors.rs`, `tests/ports.rs`, `tests/output.rs`, `tests/kernel.rs` |
| WEB-1 | Las páginas on-demand son exactamente las declaradas | `test` | `routes.test.ts` |
| WEB-2 | Una sola fuente de rutas, sin rutas internas a mano | `test` | `routes.test.ts` |
| WEB-2 | Las redirecciones 301 apuntan a una ruta y responden 301 | `pendiente` | `REDIRECTS` está vacío en la plantilla: el test unitario y el e2e de `pages.spec.ts` recorren un conjunto vacío y no verifican nada. Falta una redirección de ejemplo |
| WEB-3 | Contenido como datos tipados | `herramienta` | `astro check`: el schema de las colecciones de contenido |
| WEB-3 | Sin texto de relleno ni contenido incrustado en componentes | `revisor:site-auditor` | Paso 6 del auditor |
| WEB-4 | Accesibilidad automática en cada página | `test` | `pages.spec.ts` (axe) |
| WEB-4 | Textos alternativos y estructura | `revisor:site-auditor` | Pasos 1 y 2 del auditor |
| WEB-5 | Presupuestos de rendimiento | `herramienta` | Lighthouse CI (`lighthouserc.cjs`), job `astro-e2e` |
| WEB-6 | La CSP no permite `unsafe-inline` ni `unsafe-eval` | `test` | `security.test.ts` |
| WEB-6 | Sin violaciones de CSP en las páginas servidas | `test` | `pages.spec.ts` |
| WEB-6 | Sin `style=""` ni manejadores inline | `revisor:site-auditor` | Paso 7 del auditor |
| WEB-7 | Sin solicitudes a terceros | `test` | `pages.spec.ts` |
| WEB-7 | El aviso de privacidad coincide con los proveedores reales | `revisor:site-auditor` | Paso 5 del auditor |
| WEB-8 | Reglas puras y antispam | `test` | `contact.test.ts` |
| WEB-8 | Funciona sin JavaScript | `test` | `contact.spec.ts` |
| WEB-9 | Metadatos, sitemap y robots | `test` | `pages.spec.ts` |
| WEB-9 | Las rutas `noindex` quedan fuera del sitemap | `test` | `routes.test.ts` |
| WEB-9 | Descripciones, títulos y JSON-LD | `revisor:site-auditor` | Pasos 3 y 4 del auditor |
| WEB-10 | `.gitignore` y `.env.example` sin secretos | `test` | `env.test.ts` |
| WEB-10 | Secretos declarados en un schema | `herramienta` | `astro check` y el build: `env.schema` de `astro.config.mjs` |
| AS-1 | Astro estático sobre Cloudflare Workers | `herramienta` | `astro build` con el adaptador de Cloudflare, servido por `wrangler dev` en el job `astro-e2e` |
| AS-2 | El CSS de Tailwind sale en archivo y respeta la CSP | `test` | `pages.spec.ts` |
| AS-2 | Los tokens de marca viven en `@theme` | `pendiente` | Nada comprueba que los colores y tipografías salgan de `@theme` y no de valores sueltos. Automatizable con una regla de lint sobre las clases arbitrarias |
| AS-3 | Código en inglés; contenido, URLs y mensajes en el idioma del cliente | `revisor:site-auditor` | Paso 9 del auditor |
| AS-4 | CSP con los hashes de Astro y una sola fuente de cabeceras | `test` | `security.test.ts` |
| AS-5 | Formulario con Action y confirmación on-demand | `test` | `contact.spec.ts` |
| AS-5 | Las reglas del formulario son puras | `test` | `contact.test.ts` |
| AS-6 | E2E contra el build real en `wrangler dev` | `herramienta` | `playwright.config.ts` levanta `wrangler dev`, job `astro-e2e` |
