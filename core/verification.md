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
| HEX-1 | Autenticación: ningún contexto depende del de identidad; la cuenta cruza como tipo del borde | `test` | `tests/dependencies.rs::hex_1_dependencies_point_inward` (un `{ctx}-api` que declara `iam-application` falla) |
| HEX-2 | Núcleo solo con el estándar y el kernel | `test` | `tests/dependencies.rs::hex_2_core_depends_only_on_std_and_the_kernel` |
| HEX-2 | Frameworks de servidor solo en el borde | `test` | `tests/dependencies.rs::hex_2_server_frameworks_only_at_the_edge` |
| HEX-3 | El dominio no usa las primitivas de E/S del kernel | `test` | `tests/ports.rs::hex_3_domain_does_not_use_kernel_io_primitives` |
| HEX-3 | El dominio no declara asincronía propia (`async fn`, bloques y closures `async`, `.await`, `std::future`/`std::task`, `Future`, `AsyncFn*`; también en macros y en `#[cfg(test)]`) | `test` | `tests/ports.rs::hex_3_domain_declares_no_asynchrony` |
| HEX-4 | Un `UseCase` de un solo `execute` | `test` | `tests/naming.rs::hex_4_inbound_ports_are_single_method_use_cases` |
| HEX-4 | Las implementaciones se llaman `*Handler` | `test` | `tests/naming.rs::hex_4_use_case_implementations_are_handlers` |
| HEX-4 | `api` depende del `UseCase`, nunca del `Handler` | `test` | `tests/naming.rs::hex_4_api_depends_on_use_cases_never_on_handlers` |
| HEX-4 | El contrato no expone tipos de `domain` | `test` | `tests/naming.rs::hex_4_use_case_contracts_do_not_expose_domain_types` |
| HEX-4 | Suscriptores: `*EventSubscriber` con solo `on`; `*Subscriber` en `subscribers/`, sin `UseCase` | `test` | `tests/naming.rs::hex_4_event_subscribers_are_named_and_placed` y `hex_4_inbound_ports_are_single_method_use_cases` |
| HEX-4 | Workflows en `workflows/`, sin `UseCase`, invisibles para `api` | `test` | `tests/naming.rs::hex_4_workflows_are_internal_to_application` |
| HEX-5 | Puertos salientes con sufijo de rol o nombre propio de la lista (`UnitOfWork`), nunca `Port` | `test` | `tests/naming.rs::hex_5_outbound_ports_are_named_by_role` |
| HEX-5 | Adaptadores `{Tecnología}{Concepto}Adapter` en infraestructura, sin prefijos de doble | `test` | `tests/naming.rs::hex_5_adapters_are_named_by_technology_and_concept` |
| HEX-6 | DTOs de puerto saliente sin `Command`, `Query`, `Response` ni `Result` | `test` | `tests/naming.rs::hex_6_outbound_dtos_do_not_use_reserved_vocabulary` |
| HEX-6 | Tipos de transporte con el nombre del transporte | `test` | `tests/naming.rs::hex_6_transport_types_carry_the_transport_name` |
| HEX-7 | Lógica de negocio fuera del dominio | `revisor:architecture-reviewer` | Prioridad 1 del revisor: `if` de negocio, cálculos y transiciones en handlers o adaptadores |
| HEX-7 | Eventos: los registra el agregado; el handler publica después de guardar y no los persiste | `revisor:architecture-reviewer` | Prioridad 1 del revisor; el despachador fuera de `application` lo cubre `tests/naming.rs::hex_5_adapters_are_named_by_technology_and_concept` |
| HEX-7 | Servicio de dominio sin estado, sin puertos ni E/S | `revisor:architecture-reviewer` | Prioridad 1 del revisor |
| HEX-7 | Ningún tipo de `domain` ni de `application` (fuera de `ports/outbound`) se llama `…Service` | `test` | `tests/naming.rs::hex_7_service_suffix_is_only_an_outbound_role` |
| HEX-8 | El kernel no tiene dependencias | `test` | `tests/kernel.rs::hex_8_kernel_depends_only_on_std` |
| HEX-8 | Traits del kernel: marcadores o capacidades de la lista cerrada | `test` | `tests/kernel.rs::hex_8_kernel_traits_are_model_markers_or_listed_capabilities` |
| ERR-1 | Los errores de `domain` no implementan `std::error::Error` | `test` | `tests/errors.rs::err_1_domain_errors_do_not_implement_std_error` |
| ERR-1 | Los errores de `application` (hojas, contratos, puertos) tampoco | `test` | `tests/errors.rs::err_1_application_errors_do_not_implement_std_error` |
| ERR-1 | Excepciones para fallos de negocio, conversiones amplias entre árboles | `revisor:architecture-reviewer` | Prioridad 3 del revisor |
| ERR-2 | Exhaustividad del contrato en el borde | `herramienta` | Compilador: `match` exhaustivo sobre `{Verb}{Noun}Error` en `api` |
| ERR-2 | Un test por variante admitida del contrato | `revisor:architecture-reviewer` | Prioridad 3 del revisor: por cada variante de `{Verb}{Noun}Error`, un test del handler que la produce y uno del endpoint que la traduce |
| ERR-2 | Corte: un estado de negocio persistido con éxito no es una hoja de error | `revisor:architecture-reviewer` | Prioridad 3 del revisor |
| ERR-3 | Un caso por resultado en la batería de contrato, uno por variante declarada | `herramienta` | Compilador: `scenario_of` hace un `match` exhaustivo sobre `ModerationError`, y `moderation_gateway_contract!` exige un campo por caso. `cargo test` corre la batería contra el doble y cada adaptador. Sus casos los genera una macro: el verificador no puede citarlos |
| ERR-3 | Semántica: rechazo como éxito, sin errores pensados para el transporte | `revisor:architecture-reviewer` | Prioridad 2 del revisor |
| ERR-3 | Éxito degradado: un colaborador posterior a la confirmación no revierte el caso de uso | `revisor:architecture-reviewer` | Prioridad 2 del revisor |
| ERR-4 | El núcleo no escribe en la salida | `test` | `tests/output.rs::err_4_core_does_not_write_to_output` |
| ERR-4 | El núcleo no importa transporte ni logging | `test` | `tests/dependencies.rs::hex_2_core_depends_only_on_std_and_the_kernel` |
| ERR-4 | Sin datos sensibles en errores, mensajes ni registros | `revisor:architecture-reviewer` | Prioridad 4 del revisor |
| ERR-4 | El registro de negocio va por un puerto; el `FaultReporter` solo desde el manejador global | `revisor:architecture-reviewer` | Prioridad 4 del revisor |
| TDD-1 | Test antes o junto a la producción (backend) | `revisor:architecture-reviewer` | Diff separado en producción y test, y orden de los commits |
| TDD-1 | Test antes o junto a la producción (sitios) | `revisor:site-auditor` | Paso 8 del auditor |
| TDD-2 | Los tests del núcleo no levantan runtime ni framework | `test` | `tests/dependencies.rs::hex_2_core_depends_only_on_std_and_the_kernel` (lee `[dev-dependencies]`) |
| TDD-2 | Dobles de puertos entrantes: `{Fake\|Fixed\|Stub\|Spy}{Puerto}`, sin `Adapter` | `test` | `tests/naming.rs::tdd_2_inbound_doubles_are_named_by_prefix_and_port` |
| TDD-2 | Dobles con prefijo `Fake`, `Fixed`, `Stub` o `Spy` (presencia del prefijo) | `test` | `tests/naming.rs::hex_5_adapters_are_named_by_technology_and_concept` |
| TDD-2 | Semántica de los prefijos: `Fake` con estado, `Fixed` y `Stub` con respuesta fija, `Spy` registra las llamadas | `revisor:architecture-reviewer` | Prioridad 5 del revisor: el prefijo de cada doble del diff dice lo que hace |
| TDD-2 | Doble y adaptadores cumplen el mismo contrato | `herramienta` | `cargo test`: las baterías `{puerto}_contract!` corren contra el doble y cada adaptador. Sus casos los genera una macro: el verificador no puede citarlos |
| TDD-1 | La propagación de una firma que pasa a ser falible llega con su test, después del commit de la firma | `revisor:architecture-reviewer` | Prioridad 5 del revisor: orden de los commits |
| TDD-2 | Una prueba de falla por cada llamada a un puerto que puede fallar | `revisor:architecture-reviewer` | Prioridad 5 del revisor. Ejemplo en la plantilla: `fails_with_unexpected_when_save_fails` y `fails_with_unexpected_when_publish_fails` (`failing_on`) |
| TDD-2 | Forma de los tests: sin contenedor, fakes en vez de mocks, reglas no probadas por el endpoint | `revisor:architecture-reviewer` | Prioridad 5 del revisor |
| VER-1 | Los tests de arquitectura corren en CI y no pasan en vacío | `herramienta` | `.github/workflows/harness.yml`, job `rust`, con `ARCH_REQUIRE_MATERIAL=1` |
| VER-1 | Lo citado existe y cada regla tiene mecanismo | `herramienta` | `scripts/check_harness.py` (`MISSING-TEST` y `VERIF-*`), job `consistency` |
| VER-1 | El trinquete rechaza claves nuevas o por encima de su conteo, y pide borrar o bajar lo que ya no viola | `test` | `tests/ratchet.rs::ver_1_baseline_rejects_a_new_violation`, `tests/ratchet.rs::ver_1_baseline_rejects_one_more_violation_in_a_listed_file`, `tests/ratchet.rs::ver_1_baseline_asks_to_delete_what_no_longer_violates` |
| VER-1 | Los ids del trinquete son los nombres de los tests | `test` | `tests/ratchet.rs::ver_1_baseline_ids_match_tests` |
| VER-1 | Una edición del agente no agranda `baseline.txt` | `herramienta` | Hook `PreToolUse` de craft-backend (`craft_hooks.py pre-edit`, tests en `scripts/tests/test_craft_hooks.py`): bloquea un Edit, Write o MultiEdit que agregue o suba entradas, y uno cuyo resultado no puede calcular |
| VER-1 | Una exención de material solo cubre un conjunto vacío del test que nombra, con motivo, y se pide borrar cuando deja de estar vacío | `test` | `tests/ratchet.rs::ver_1_material_exemptions_read_only_their_test`, `tests/ratchet.rs::ver_1_material_exempt_empty_set_passes`, `tests/ratchet.rs::ver_1_material_empty_set_without_exemption_fails`, `tests/ratchet.rs::ver_1_material_exemption_of_a_non_empty_set_asks_to_delete_it`, `tests/ratchet.rs::ver_1_material_exemption_of_an_unknown_set_fails`, `tests/ratchet.rs::ver_1_material_exemption_problems_name_the_line`, `tests/ratchet.rs::ver_1_material_exemptions_name_tests`, `tests/ratchet.rs::ver_1_verdict_reads_the_exemptions_of_the_directory` |
| VER-1 | El agente no agrega exenciones de material | `herramienta` | Hook `PreToolUse` de craft-backend (`craft_hooks.py pre-edit`, `PreEditExemptions` en `scripts/tests/test_craft_hooks.py`): bloquea una exención nueva en `material-exempt.txt`, también con `CRAFT_BACKEND_BASELINE_INIT=1` |
| VER-1 | Ningún commit agranda `baseline.txt` ni agrega exenciones a `material-exempt.txt` (shell, a mano) | `pendiente` | El hook solo ve las herramientas de edición del agente y corre donde está el plugin. Automatizable con un paso de CI del proyecto que compare la línea base con la rama base |
| RS-1 | Un crate por capa; Cargo hace cumplir las dependencias | `test` | `tests/dependencies.rs::hex_1_dependencies_point_inward` |
| RS-1 | Las dependencias de cada miembro heredan del workspace (`workspace = true`) | `test` | `tests/dependencies.rs::rs_1_dependencies_inherit_from_the_workspace` |
| RS-2 | `unwrap` prohibido | `herramienta` | Clippy, `unwrap_used = "deny"` en `[workspace.lints]` |
| RS-2 | `panic!` y `expect` solo para invariantes rotos | `revisor:architecture-reviewer` | Prioridad 6 del revisor |
| RS-3 | Puertos async con `BoxFuture`, sin `async-trait` en el núcleo | `test` | `tests/dependencies.rs::hex_2_core_depends_only_on_std_and_the_kernel` |
| RS-4 | Tests de arquitectura con `toml` y `syn` | `test` | `tests/naming.rs`, `tests/dependencies.rs`, `tests/errors.rs`, `tests/ports.rs`, `tests/output.rs`, `tests/kernel.rs` |
| WEB-1 | Las páginas on-demand son exactamente las declaradas | `test` | `routes.test.ts` |
| WEB-2 | Una sola fuente de rutas, sin rutas internas a mano | `test` | `routes.test.ts` |
| WEB-2 | Las redirecciones 301 apuntan a una ruta y responden 301 | `test` | `routes.test.ts` y `pages.spec.ts`, sobre la redirección de ejemplo; con `WEB_REQUIRE_MATERIAL=1` en el CI del harness, un `REDIRECTS` vacío falla |
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
| AS-2 | Los tokens de marca viven en `@theme` | `test` | `theme.test.ts`: clases con colores o tipografías fuera de `@theme` y colores escritos a mano en CSS |
| AS-3 | Código en inglés; contenido, URLs y mensajes en el idioma del cliente | `revisor:site-auditor` | Paso 9 del auditor |
| AS-4 | CSP con los hashes de Astro y una sola fuente de cabeceras | `test` | `security.test.ts` |
| AS-5 | Formulario con Action y confirmación on-demand | `test` | `contact.spec.ts` |
| AS-5 | Las reglas del formulario son puras | `test` | `contact.test.ts` |
| AS-6 | E2E contra el build real en `wrangler dev` | `herramienta` | `playwright.config.ts` levanta `wrangler dev`, job `astro-e2e` |
