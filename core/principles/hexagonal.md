# Principios — Arquitectura hexagonal

> **Estado:** v0.2
> **Origen:** destilado de un proyecto Java 21 + Spring Boot con 7 ADR. Validado en Rust ([stacks/rust](../../stacks/rust/README.md): plantilla y migración de `communities` en kumenity-backend-rust). Pendiente de validar en Kotlin y TypeScript.
> **Relacionados:** [errors.md](errors.md) (`ERR`), [tdd.md](tdd.md) (`TDD`), [verification.md](verification.md) (`VER`).

Formato de cada principio: ver [README](README.md).

---

## Disposición de referencia

```
{contexto}/
├── domain/                        Modelos y reglas de negocio. Sin puertos.
├── application/
│   ├── ports/
│   │   ├── inbound/               {Verb}{Noun}UseCase
│   │   └── outbound/              {Concepto}{Rol}  (Repository, Gateway, …)
│   │       └── dtos/              {Contrato}Payloads
│   ├── commands/{caso}/           Command, Response, Error, Handler
│   ├── queries/{caso}/            Query, Response, Error, Handler
│   └── errors/                    Hojas de error compartidas del contexto (ERR-2)
├── api/                           Adaptadores driving. Solo ven application.
├── infrastructure/                Adaptadores driven.
└── bootstrap/                     Composition root.
```

La forma concreta (paquetes, módulos, crates, carpetas) depende del stack; los
nombres de capa y la dirección de dependencias no.

---

## HEX-1 — Anillos concéntricos, dependencias hacia adentro

**Regla.**

- Las capas son, de adentro hacia afuera: `domain` → `application` → `api` | `infrastructure`. Una capa solo depende de capas interiores.
- `api` (adaptadores *driving*) depende **solo** de `application`, nunca de `domain`.
- `infrastructure` (adaptadores *driven*) puede depender de `application` y de `domain`.
- `api` e `infrastructure` son pares del mismo anillo y **no se importan entre sí**.
- El punto de arranque (`bootstrap`) puede verlo todo, pero nadie depende de él.

**Fundamento.** La dirección de dependencias es lo que hace sustituibles los adaptadores. Si HTTP conoce la persistencia, o viceversa, el anillo exterior se acopla consigo mismo y cambiar uno arrastra al otro. `api` no ve `domain` porque el contrato del caso de uso (HEX-4) es la única superficie que el transporte debe conocer: si el controller puede tocar el modelo, termina serializando agregados o duplicando reglas. `infrastructure` sí lo ve porque persistir o sincronizar un agregado exige conocer su forma.

**Violaciones típicas.** Un controller que inyecta un repositorio para "ahorrarse" un caso de uso. Un adaptador de persistencia que devuelve un DTO HTTP. Un controller que importa un value object del dominio para construir el `Command`.

**Verificación.** Test de arquitectura que falla el build (`VER-1`): `api` no importa `domain` ni `infrastructure`; `infrastructure` no importa `api`.

---

## HEX-2 — Núcleo libre de framework; el cableado vive en el composition root

**Regla.** `domain` y `application` dependen solo del lenguaje estándar, de sus propios paquetes y del kernel compartido (HEX-8). Las implementaciones de caso de uso son objetos planos con inyección por constructor. Todo el cableado ocurre en un único **composition root** en el anillo exterior, que instancia los handlers y expone **el contrato**, no la implementación.

**Fundamento.** Tests del núcleo sin levantar contenedor (instancias puras + fakes). El framework es un detalle del borde: cambiar de runtime, de IoC o de stack HTTP solo reescribe el composition root y los adaptadores.

**Violaciones típicas.** Anotaciones del framework en un handler "porque es más cómodo". Librerías utilitarias (Lombok, Vavr, lodash) colándose en el dominio. Un composition root que devuelve el tipo concreto.

**Verificación.** Test de arquitectura: el núcleo no importa nada fuera de la lista permitida.

---

## HEX-3 — La frontera hexagonal vive en `application`; el dominio no tiene puertos

**Regla.** Puertos entrantes (`ports/inbound`) y salientes (`ports/outbound`) se declaran en `application`. `domain` contiene solo modelos y reglas de negocio.

Única excepción de ubicación: las capacidades técnicas transversales de la lista cerrada de HEX-8 (`Clock`, `IdGenerator`, `FaultReporter`), que viven en el kernel.

**Fundamento.** La aplicación es la frontera lógica del caso de uso: declara qué ofrece y qué necesita. Persistencia e integraciones son detalle de orquestación, no reglas de negocio. Un dominio sin puertos no cambia cuando cambia la frontera técnica, y no conoce quién lo invoca.

> **Nota de precisión teórica.** Esto diverge conscientemente de Evans (*DDD*, 2003), que ubica las interfaces de `Repository` en la capa de dominio. Es coherente con Cockburn, donde los puertos pertenecen al borde de la *aplicación*. La divergencia debe quedar explícita para que nadie la "corrija" citando a Evans.

**Violaciones típicas.** Crear `domain/ports` por costumbre. Un agregado que recibe un repositorio para cargarse a sí mismo.

**Verificación.** Test de arquitectura: ningún tipo de `domain` es una interfaz de puerto; `domain` no depende de `application`. El nombre no distingue un puerto de una abstracción de dominio legítima (una política, una especificación); la semántica sí: un puerto implica E/S. El test comprueba que `domain` no use las primitivas de E/S del stack (asincronía, falla de infraestructura).

---

## HEX-4 — Puertos entrantes: un caso de uso, una interfaz

**Regla.**

- Un puerto entrante por caso de uso: `{Verb}{Noun}UseCase`, con un único método `execute`, declarado en `application/ports/inbound`.
- La implementación se llama `{Verb}{Noun}Handler`. Contrato e implementación tienen nombres distintos.
- Separación comando / consulta: escrituras en `application/commands/{caso}`, lecturas en `application/queries/{caso}`. Cada caso agrupa su `Command` o `Query`, su `Response` (si hay payload), su `Error` (si tiene árbol A, ver `ERR-2`) y su `Handler`.
- Los DTOs del contrato son tipos de primer nivel junto al handler, no anidados en la interfaz.
- El contrato (`Command`, `Query`, `Response`, `Error`) no expone tipos de `domain`: como `api` no ve `domain` (HEX-1), el contrato se expresa con tipos del lenguaje o de `application`.
- El adaptador *driving* depende del `UseCase`, nunca del `Handler`.

**Fundamento.** Un caso de uso es la unidad de cambio y de prueba. Interfaces de un solo método evitan "servicios de aplicación" que crecen sin límite. El adaptador depende de una abstracción, así que se puede sustituir o probar sin la clase concreta.

**Violaciones típicas.** Un `CommunityService` con diez métodos. Un controller que inyecta el handler. Un `Response` que expone el agregado tal cual.

**Verificación.** Test de arquitectura: `api` no depende de `*Handler` y cada `*UseCase` declara un solo método. Tampoco expone tipos de `domain`: ni en los campos públicos y variantes de `Command`, `Query`, `Response` y `Error`, ni en las firmas del `UseCase`, ni en una reexportación pública desde `application`. Las conversiones (`From<&Agregado> for Response`) y los `use` del handler no son contrato. Que `api` no declare `domain` no basta: un valor de dominio que llega por un campo público se puede usar sin importar el crate. En Rust: `tests/naming.rs::hex_4_*`.

---

## HEX-5 — Puertos salientes nombrados por su rol

**Regla.** El sufijo del puerto declara qué tipo de colaborador es. El sufijo genérico `Port` está prohibido.

| Sufijo | Rol |
|--------|-----|
| `Repository` | Persistencia durable |
| `Cache` | Datos volátiles (TTL, uso único, no persistidos) |
| `Gateway` | Servicio de un tercero |
| `Client` | Otro servicio del mismo sistema |
| `Synchronizer` | Publicar o sincronizar estado hacia otro sistema |
| `Service` | Utilidad que no encaja en lo anterior (último recurso) |

Los adaptadores se nombran `{Tecnología}{Concepto}Adapter`, donde **Concepto es el sustantivo del puerto sin el sufijo de rol** (`Postgres…`, `InMemory…`, `Http…`, `Stub…`, `NoOp…`):

| Puerto | Adaptador |
|--------|-----------|
| `CommunityRepository` | `PostgresCommunityAdapter`, `InMemoryCommunityAdapter` |
| `ModerationGateway` | `HttpModerationAdapter`, `StubModerationAdapter` |

Única excepción: las capacidades técnicas transversales del kernel (HEX-8) no llevan sufijo de rol. Sus adaptadores sí siguen esta regla, con el nombre completo de la capacidad como Concepto: `UuidIdGeneratorAdapter`, `SystemClockAdapter`.

**Fundamento.** Al leer la firma de un handler se sabe qué toca: una base de datos, un tercero, un servicio propio. `Port` es uniforme pero opaco. En el adaptador, el rol ya está implícito en la tecnología (`Postgres` es persistencia, `Http` es remoto); repetirlo solo alarga el nombre.

**Violaciones típicas.** `Service` usado como cajón de sastre. Un `Gateway` para un servicio propio. `PostgresCommunityRepositoryAdapter` (repite el rol).

**Verificación.** Tests de arquitectura, uno por mitad de la regla. Los puertos de `ports/outbound` llevan un sufijo de rol y nunca `Port`. En `infrastructure`, todo tipo que implementa un puerto de su contexto o una capacidad del kernel se llama `{Prefijo}{Concepto}Adapter`, con prefijo no vacío. Así falla `PostgresCommunityRepositoryAdapter`, que repite el rol. En Rust: `tests/naming.rs::hex_5_*`.

---

## HEX-6 — Vocabulario de tipos sin colisiones

**Regla.**

- Los DTOs de un puerto saliente no se anidan en la interfaz: viven en `ports/outbound/dtos`, agrupados en un contenedor por contrato (`{Contrato}Payloads`).
- Cada frontera tiene su vocabulario, y ningún sufijo se repite entre fronteras:

| Frontera | Entrada | Salida | Dónde |
|----------|---------|--------|-------|
| Caso de uso | `Command` / `Query` | `Response` | `application/commands`, `application/queries` |
| Puerto saliente | `Request` (`ReviewContentRequest`) | Nombre del concepto, sin sufijo (`ModerationVerdict`, `UserProfile`) | `application/ports/outbound/dtos` |
| Transporte | `{Caso}{Transporte}Request` | `{Caso}{Transporte}Response` | `api` |
| Control de flujo | — | `Result` (`ERR-1`) | kernel |

- Para HTTP: `{Caso}HttpRequest` y `{Caso}HttpResponse`. Otros transportes siguen el patrón (`{Caso}GrpcRequest`, `{Caso}Message`).
- Un puerto cuyas firmas solo usan tipos de dominio no gana un contenedor de payloads vacío.

**Fundamento.** La interfaz queda como lista de operaciones. Un vocabulario sin colisiones hace que el nombre de un tipo diga a qué frontera pertenece. Los tipos de transporte llevan el nombre del transporte porque son los únicos que cambian si cambia el transporte.

**Violaciones típicas.** `ReviewContentCommand` como DTO de un gateway, en lugar de `ReviewContentRequest`. `ModerationResult` como payload de éxito, en lugar de `ModerationVerdict`. Un controller que recibe un `CreateCommunityRequest` (colisiona con el vocabulario del puerto saliente) o devuelve el `Response` del caso de uso tal cual.

**Verificación.** Test de arquitectura: ningún tipo en `ports/outbound/dtos` termina en `Command`, `Query`, `Response` ni `Result`; en `api`, los tipos `*Request`/`*Response` llevan el nombre del transporte.

---

## HEX-7 — El dominio decide; la aplicación orquesta

**Regla.**

| Capa | Sí | No |
|------|----|----|
| Dominio | Validaciones, invariantes, transiciones de estado, reglas | Puertos, framework, mapeo de transporte o persistencia |
| Aplicación | Leer el input, cargar, invocar al dominio, llamar puertos salientes, mapear la respuesta | `if` de negocio, validar campos del agregado, reglas de estado |
| Api | Forma del transporte (validación sintáctica del payload) | Duplicar reglas de dominio |

Agregados inmutables con comportamiento en sus métodos. Las reglas se evalúan en el dominio y devuelven `Result` (`ERR-1`).

**Fundamento.** Un handler con lógica de negocio es un modelo anémico disfrazado. Si la regla vive en el agregado, se prueba sin infraestructura y no se duplica entre casos de uso.

**Violaciones típicas.** El handler calcula, valida longitudes o decide si una transición es válida. Es **la violación más frecuente de los agentes de IA**: tienden a escribir el `if` donde están editando.

**Verificación.** Revisión (difícil de automatizar). Cubierto por el subagente `architecture-reviewer`.

---

## HEX-8 — Kernel compartido delgado

**Regla.** Un módulo de kernel compartido contiene **solo primitivas**: `Result`, marcadores de modelo (`ValueObject`, `Entity`, `AggregateRoot`, `Identifier`), marcadores de error y de evento, y las **capacidades técnicas transversales** de la lista de abajo. Nunca agregados de un contexto, puertos de un caso de uso ni datos sensibles. Sin framework ni dependencias: solo el lenguaje estándar. Si el kernel se divide en capas, HEX-1 rige dentro de él.

Las jerarquías cerradas de cada contexto viven en el contexto, no en el kernel (los marcadores del kernel son abiertos).

**Capacidades técnicas transversales.** Son los únicos puertos que viven fuera de `application/ports`. Una capacidad entra solo si cumple **todos** estos criterios:

- Es una capacidad técnica, sin vocabulario de negocio.
- La necesita más de un contexto.
- Su firma se expresa solo con tipos del lenguaje estándar y del kernel.

La lista es **cerrada**:

| Capacidad | Qué hace |
|-----------|----------|
| `Clock` | Da la hora actual |
| `IdGenerator` | Genera identificadores |
| `FaultReporter` | Registra las fallas del árbol B desde el manejador global (`ERR-4`) |

Agregar una exige cambiar esta lista y el test del kernel en el mismo commit.

- **Nombre:** sustantivo de agente por lo que hace, sin sufijo de rol y nunca `Port`. Es la única excepción a HEX-5 para puertos y la única a HEX-3 para su ubicación.
- **Adaptadores:** siguen HEX-5. Como la capacidad no tiene sufijo de rol que quitar, el Concepto es su nombre completo: `UuidIdGeneratorAdapter`, `SystemClockAdapter`.

**Fundamento.** Un sitio único para los bloques de construcción evita copiar contratos sin acoplar contextos entre sí. Si crece, se vuelve un acoplamiento disfrazado. Por eso las capacidades tienen criterio y lista cerrada: "varios contextos lo usan" no basta, porque un puerto de negocio compartido es justo el acoplamiento que el kernel debe evitar. Sin sufijo de rol, porque el rol de una capacidad no varía. Un `Clock` no es ni persistencia ni un tercero: es el reloj.

**Violaciones típicas.** Mover un agregado al kernel "porque dos servicios lo usan". Agregar un puerto con vocabulario de negocio (`CommunityLookup`) al kernel para compartirlo entre contextos. Una capacidad que no está en la lista. Un adaptador de capacidad sin sufijo (`UuidIdGenerator`).

**Verificación.** Test de arquitectura propio del kernel: no tiene dependencias, y todo trait público es un marcador de modelo o una capacidad de la lista. Los nombres de los adaptadores los cubre el test de HEX-5. En Rust: `tests/kernel.rs::hex_8_*` y `tests/naming.rs::hex_5_adapters_*`.

---

## Antipatrones

| En vez de | Usar |
|-----------|------|
| Puertos en `domain` | `application/ports/{inbound,outbound}` |
| Sufijo `Port` en puertos salientes | Sufijo de rol (HEX-5) |
| DTOs anidados en la interfaz del puerto | `ports/outbound/dtos/{Contrato}Payloads` |
| DTO de puerto con sufijo `Command`, `Response` o `Result` | `Request` / nombre del concepto |
| `Request`/`Response` genéricos en `api` | `{Caso}HttpRequest` / `{Caso}HttpResponse` |
| Servicio de aplicación con muchos métodos | Un `UseCase` + `Handler` por caso de uso |
| Adaptador *driving* que inyecta el handler | Inyectar el `UseCase` |
| `api` importando tipos de `domain` | Contrato del caso de uso sin tipos de dominio |
| Framework o librerías utilitarias en el núcleo | Estándar del lenguaje + kernel |
| Agregar un trait al kernel porque varios contextos lo usan | Solo capacidades de la lista de HEX-8 |
| Reglas de negocio en el handler | Encapsularlas en el dominio |

---

## Decisiones abiertas

1. **Criterio para el sufijo `Service`.** Definir cuándo se permite, para que no se vuelva cajón de sastre.
