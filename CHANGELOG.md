# Changelog

## Sin publicar

### Cambiado
- Gate de Stop de `craft-backend` y `craft-web`: el mensaje ya no empuja a
  "corregir" cualquier rojo. Distingue dos casos: si el fallo lo causó el cambio
  del agente, lo corrige (VER-1); si el rojo es intencional (fase roja de TDD) o
  el agente cree que el test está mal, no lo modifica, explica el estado y
  termina (TDD-1). La skill `tdd` y los README de `stacks/rust` y `stacks/astro`
  dicen lo mismo con las mismas palabras.
- Plantilla Rust: los errores de dominio (`domain_error!`) ya no implementan
  `std::error::Error`, solo `Debug` y `Display` (ERR-1, RS-2). Antes, en una
  función que devuelve `Result<_, Fault>`, un `?` sobre un error de dominio lo
  convertía en `Fault` por el `impl From<E: Error> for Fault`, y el fallo de
  negocio terminaba como un 500 genérico. Ningún código dependía de esa
  conversión: el workspace compiló sin cambios.
- `core/principles/errors.md`, ERR-1: ningún error del árbol A pasa al árbol B
  de forma implícita, y se agrega la violación típica correspondiente.
- `stacks/rust/README.md`: fila "Error de dominio" en la tabla de RS-2, el
  párrafo sobre `Fault` corregido (el compilador obliga a traducir tanto hacia
  `Failure<E>` como hacia `Fault`), la regla en Enforcement y la fila en
  "Violaciones típicas". Skill `rust-stack`: la regla en el bullet de RS-2.

### Agregado
- Plantilla Rust: test de arquitectura
  `tests/errors.rs::err_1_domain_errors_do_not_implement_std_error` (`syn`).
  Detecta `impl Error for …` en los crates `domain` con cualquier ruta
  (`Error`, `std::error::Error`, `::core::error::Error`, un alias `use … as`),
  incluido el cuerpo de un `macro_rules!`. Informa la línea, para lo que se
  activa `span-locations` de `proc-macro2`. La plantilla pasa a 36 tests.
- `core/principles/web.md`: principios WEB-1 a WEB-10 para sitios informativos
  (estático, rutas, contenido tipado, a11y, rendimiento, CSP, privacidad,
  formularios, SEO, secretos). Destilados de kumelemuel.dev y kidus-akaal.com.
- `stacks/astro/`: guía con las decisiones AS-1 a AS-6 y plantilla ejecutable
  (Astro 7 + Cloudflare Workers + Tailwind 4). Build y `astro check` limpios,
  21 tests unitarios y 16 e2e. Los e2e corren contra el build real en
  `wrangler dev` e incluyen axe, CSP y terceros. Las reglas se verificaron
  inyectando una violación de cada tipo.

- Plugin `craft-web` 0.1.0:
  - Skills: `web-principles` (WEB-1..10), `astro-stack` (cómo en la plantilla)
    y `client-delivery` (descubrimiento → spec → cotización → entrega).
  - Subagente `site-auditor`: alt, encabezados, SEO, JSON-LD, coherencia del
    aviso de privacidad.
  - Hooks: `PostToolUse` (Prettier + vitest, ~5 s) y `Stop` (eslint, astro
    check y vitest, ~7 s), solo en proyectos con `src/routes.ts` y con el Node
    de `.nvmrc`.
- `templates/web/`: cuestionario de descubrimiento, spec del sitio, base de
  cotización y checklist de entrega.
- Plantilla Astro: test WEB-2 que impide escribir `href="/…"` a mano.

### Verificado
- Las tres skills se activan solas ante preguntas que no las nombran.
- En una sesión real sin `AGENTS.md` ni skills, el hook bloqueó una ruta
  escrita a mano y el agente la corrigió a `route("contact")`. Con
  `AGENTS.md`, el agente la evitó desde el principio.
- Gate de Stop vs. TDD-1, simulado con un test roto: con
  `stop_hook_active: false` bloquea (exit 2) con el mensaje nuevo completo; con
  `true` pasa (exit 0). En ambos plugins.
- Gate de Stop vs. TDD-1, sesiones reales en copias de las plantillas pidiendo
  "escribe solo el test que falla … y detente":
  - Rust (`CommunityName` colapsa espacios internos): el gate bloqueó una vez.
    El agente explicó que el rojo era intencional y terminó sin tocar el test
    ni `CommunityName::new`.
  - Astro (`contactSubject` recorta el nombre a 60): el `post-edit` y el gate
    bloquearon una vez cada uno. El agente terminó en rojo sin tocar el test ni
    `src/lib/contact.ts`.
- ERR-1/RS-2, inyectando violaciones en una copia de la plantilla:
  1. Restaurar `impl Error for $name {}` en `domain_error!`: el test falla con
     la línea `ERR-1`. En una sesión real, el `post-edit` la devolvió al agente.
     En una primera sesión, el agente se negó a editar solo con leer el
     comentario del módulo, y hubo que ordenárselo explícitamente.
  2. `fn f() -> Result<(), Fault> { CommunitySlug::new("no valido".into())?; … }`
     en `application`: no compila (E0277, `InvalidCommunitySlug: std::error::Error`
     is not satisfied).
  3. `impl std::error::Error for X` con ruta completa: se detecta igual que la
     forma corta. También se detectan `::core::error::Error` y un alias.

### Decisiones
- Hosting por defecto: Cloudflare Workers; Vercel documentado como alternativa.
- Estilos: Tailwind 4. Código en inglés; contenido y URLs en el idioma del cliente.
- Turnstile opcional, documentado pero no incluido en la plantilla.
- El gate de Stop sigue bloqueando ante cualquier rojo: no puede saber si es
  intencional. Que bloquee una sola vez por turno (`stop_hook_active`) es lo que
  permite terminar en rojo, y el mensaje lo dice (TDD-1).
- Errores de dominio sin `std::error::Error` (ERR-1, RS-2). El compilador
  impide el `?`, y el test de arquitectura impide reintroducir el `impl` "para
  que compile", que es justo lo que sugiere el E0277. La conversión a `Fault` se
  escribe a mano y solo para datos persistidos corruptos. Todavía no hay helper
  en el kernel: no hay ningún caso en la plantilla.

## 0.5.0 — 2026-11-12

### Cambiado
- **RS-1:** los adaptadores driving viven por contexto (`contexts/{ctx}/api`,
  crate `{ctx}-api`). Lo transversal del borde HTTP (Problem Details,
  `AuthenticatedAccount`, `unexpected`) pasa al nuevo crate `edge/http`.
  `apps/api` desaparece de la plantilla; `apps/server` compone las rutas.
- Tests de arquitectura:
  - Nueva capa `Edge`. Un `{ctx}-api` solo ve su propio application.
  - `app/` en la raíz se acepta como composition root.
  - Los contextos legacy (un solo crate) se ignoran durante una migración.
  - Verificado inyectando 4 violaciones nuevas.
- ERR-4: Problem Details lleva un `code` estable como miembro de extensión.

- Hook `Stop`: clippy con `--no-deps` solo sobre los crates que siguen la
  arquitectura (misma convención de ubicación que `tools/architecture`). Los
  contextos legacy ya no bloquean por warnings; sus tests siguen corriendo.

### Agregado
- Sección "Migración incremental" en `stacks/rust`.

### Validado en un proyecto real
- Migración del contexto `communities` de kumenity-backend-rust a RS-1, con iam
  como legacy. 112 tests, clippy y fmt limpios en los crates migrados, y prueba
  de punta a punta con JWT real.

## 0.4.0 — 2026-11-12

### Agregado
- Hooks de `craft-backend` (`hooks/hooks.json` + `hooks/craft_hooks.py`):
  - `PostToolUse` sobre `Edit|Write|MultiEdit`: `rustfmt` del archivo y
    `cargo test -p architecture`. Las violaciones vuelven al agente con su ID.
  - `Stop`: si hay cambios Rust sin commit, `clippy -D warnings` y
    `cargo test --workspace`; bloquea una vez por turno (`stop_hook_active`).
  - Solo se activan en workspaces con `tools/architecture/Cargo.toml`.
  - Variables `CRAFT_BACKEND_HOOKS`, `CRAFT_BACKEND_STOP_GATE` y `CRAFT_BACKEND_HOOKS_LOG`.

### Verificado
- 8 casos simulados (archivo no Rust, fuera de workspace, edición limpia,
  violación, formateo, Stop sin cambios, Stop con test roto, `stop_hook_active`).
- Dos sesiones reales de Claude Code:
  - Se pidió explícitamente un `CommunityCachePort`. El hook lo rechazó por
    HEX-5 y el agente lo renombró a `CommunityCache`.
  - Con un bug sembrado en el dominio y una tarea sin relación, el gate de Stop
    bloqueó el cierre. El agente diagnosticó la causa y la reportó sin salirse
    del alcance pedido.

## 0.3.0 — 2026-11-12

### Agregado
- `stacks/rust/`: guía de traducción con las decisiones RS-1 a RS-4.
- `stacks/rust/template/`: workspace ejecutable (kernel, contexto `communities`,
  `api`, `server`, `tools/architecture`). 33 tests; los 9 tests de arquitectura
  se verificaron inyectando una violación por regla.
- Skill `rust-stack`, que enlaza la guía y la plantilla.

### Decisiones
- **RS-1, un crate por capa agrupado por contexto**
  (`contexts/{ctx}/{domain,application,infrastructure}`). Cargo hace cumplir la
  dirección de dependencias al compilar.
- **RS-2, árbol B en Rust:** `Fault` opaco dentro de `Failure<E>`, propagado con
  `?`; `panic!` solo para invariantes, capturado con `CatchPanicLayer`; `unwrap`
  prohibido por clippy. Cierra la decisión abierta de `errors.md`.
- **RS-3, puertos async con `BoxFuture`** del kernel: solo `std` y compatible con `dyn`.
- **RS-4, tests de arquitectura con `toml` + `syn`** en lugar de `cargo-deny`: un
  solo mecanismo cubre dependencias internas, crates externos por capa y nombres.
  Cierra la decisión abierta de `verification.md` para Rust.
- `IdGenerator` en el kernel, junto a los helpers genéricos de HEX-8, para que
  `uuid` no entre al dominio.

## 0.2.0 — 2026-11-11

### Cambiado
- Principios separados por tema, con IDs propios: `hexagonal.md` (HEX-1–8),
  `errors.md` (ERR-1–4), `tdd.md` (TDD-1–2), `verification.md` (VER-1).
- Tabla de traducción por stack movida a `stacks/README.md`.
- Skills separadas: `hexagonal-architecture`, `error-handling`, `tdd`.
- `architecture-reviewer` precarga las tres skills y cita los nuevos IDs.

### Decisiones
- **Puertos en `ports/inbound` y `ports/outbound`** (antes `ports.in`/`ports.out`):
  `in` es palabra reservada en Kotlin y Rust.
- **Tipos de transporte `{Caso}HttpRequest` / `{Caso}HttpResponse`**: elimina la
  colisión de `Response` (caso de uso) y de `Request` (puerto saliente) con el
  vocabulario de `api` (HEX-6).
- **Disposición:** handlers y su contrato en `application/commands/{caso}` y
  `application/queries/{caso}`; la interfaz `UseCase` en `application/ports/inbound`.
- **`api` solo depende de `application`** (HEX-1). Consecuencias: el contrato del
  caso de uso no expone tipos de dominio (HEX-4) y las hojas de error compartidas
  viven en `application/errors` (ERR-2).
- **Adaptadores sin el sufijo de rol** (HEX-5): `PaymentRepository` →
  `PostgresPaymentAdapter`. La tecnología ya implica el rol.

## 0.1.0 — 2026-11-11

- Primer corte vertical: principio hexagonal P1–P14, skill, subagente revisor y
  marketplace.
