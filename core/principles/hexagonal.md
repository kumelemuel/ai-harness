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
- De `application`, `api` usa solo el contrato del caso de uso: su puerto entrante, `Command`/`Query`, `Response` y `Error` (HEX-4). Nunca los puertos salientes, que son del handler.
- `infrastructure` (adaptadores *driven*) puede depender de `application` y de `domain`.
- `api` e `infrastructure` son pares del mismo anillo y **no se importan entre sí**.
- El punto de arranque (`bootstrap`) puede verlo todo, pero nadie depende de él.
- **Autenticación.** Verificar una credencial es un caso de uso del contexto de identidad (`VerifyAccessTokenUseCase`), con su puerto saliente y su adaptador como cualquier otro. Su adaptador *driving* (el middleware) vive en el `api` de ese contexto, y el composition root lo aplica a las rutas protegidas de los demás. La identidad cruza hacia los otros contextos como un tipo del borde compartido (`AuthenticatedAccount`), nunca como un tipo del contexto de identidad: ningún contexto depende de él.

**Fundamento.** La dirección de dependencias es lo que hace sustituibles los adaptadores. Si HTTP conoce la persistencia, o viceversa, el anillo exterior se acopla consigo mismo y cambiar uno arrastra al otro. `api` no ve `domain` porque el contrato del caso de uso (HEX-4) es la única superficie que el transporte debe conocer: si el controller puede tocar el modelo, termina serializando agregados o duplicando reglas. `infrastructure` sí lo ve porque persistir o sincronizar un agregado exige conocer su forma.

**Violaciones típicas.** Un controller que inyecta un repositorio para "ahorrarse" un caso de uso. Un `TokenVerifier` en un módulo compartido que ven todos los contextos, o un contexto que importa la identidad de otro para leer la cuenta autenticada. Un adaptador de persistencia que devuelve un DTO HTTP. Un controller que importa un value object del dominio para construir el `Command`.

**Verificación.** Test de arquitectura que falla el build (`VER-1`): `api` no importa `domain` ni `infrastructure`; `infrastructure` no importa `api`. Un adaptador driving tampoco nombra los puertos salientes de `application`: llama al caso de uso, no a lo que el caso de uso usa. Las dependencias declaradas no bastan cuando el adaptador comparte unidad de compilación con el dominio (un contexto sin separar), ni cuando nombra el dominio o un puerto saliente de otro contexto sin separar que su paquete declara: el test lee también los imports y las referencias del código. En Rust: `tests/dependencies.rs::hex_1_*` y `tests/naming.rs::hex_1_*`.

---

## HEX-2 — Núcleo libre de framework; el cableado vive en el composition root

**Regla.** `domain` y `application` dependen solo del lenguaje estándar, de sus propios paquetes y del kernel compartido (HEX-8). Las implementaciones de caso de uso son objetos planos con inyección por constructor. Todo el cableado ocurre en un único **composition root** en el anillo exterior, que instancia los handlers y expone **el contrato**, no la implementación.

**Fundamento.** Tests del núcleo sin levantar contenedor (instancias puras + fakes). El framework es un detalle del borde: cambiar de runtime, de IoC o de stack HTTP solo reescribe el composition root y los adaptadores.

**Violaciones típicas.** Anotaciones del framework en un handler "porque es más cómodo". Librerías utilitarias (Lombok, Vavr, lodash) colándose en el dominio. Un composition root que devuelve el tipo concreto.

**Verificación.** Test de arquitectura: el núcleo no importa nada fuera de la lista permitida. No basta con las dependencias declaradas: con un paquete ya tolerado (o, en un contexto sin separar, declarado para otra capa del mismo paquete), un import nuevo no cambia el manifiesto. El test lee también el código, archivo por archivo.

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
- **Suscriptores de eventos.** El segundo tipo de puerto entrante: `{Concepto}EventSubscriber`, en `application/ports/inbound`, con un único método `on(evento)`. No lo invoca un cliente sino el despachador de eventos de infraestructura (HEX-7), que ve `domain` (HEX-1); por eso recibe el evento de dominio, y es la única excepción a "el contrato no expone tipos de `domain`". Cada reacción es un `{Verb}{Noun}Subscriber` en `application/subscribers/`, nombrado por lo que hace (`RecordCommunityHistorySubscriber`), nunca por el evento. Orquesta como un handler. Corre dentro de la publicación, así que su falla es la del handler, en el árbol B.
- **Workflows.** Cuando varios handlers comparten la misma secuencia de pasos (reclamar un recurso, llamar a un colaborador, revertir si falla), esa secuencia es un `{Verb}{Noun}Workflow` en `application/workflows/{verb_noun}/`. No es un caso de uso: no tiene puerto entrante, el adaptador *driving* nunca lo ve, y los handlers lo reciben por constructor. Su error es un `{Verb}{Noun}WorkflowError` cerrado, hecho de las mismas hojas compartidas (`ERR-2`), que cada handler traduce con un `match` exhaustivo a las hojas que admite su contrato. Orquesta como un handler: las reglas siguen en el dominio (HEX-7).

**Fundamento.** Un caso de uso es la unidad de cambio y de prueba. Interfaces de un solo método evitan "servicios de aplicación" que crecen sin límite. El adaptador depende de una abstracción, así que se puede sustituir o probar sin la clase concreta.

**Violaciones típicas.** Un `CommunityService` con diez métodos. Un controller que inyecta el handler. Un `Response` que expone el agregado tal cual. Un handler que invoca a otro handler para reutilizar sus pasos (va en un workflow). Un controller que llama a un workflow. Un suscriptor nombrado por el evento (`CommunityCreatedSubscriber`), o una reacción escrita en el despachador en lugar de en un suscriptor.

**Verificación.** Test de arquitectura: `api` no depende de `*Handler` y cada `*UseCase` declara un solo método. Tampoco expone tipos de `domain`: ni en los campos públicos y variantes de `Command`, `Query`, `Response` y `Error`, ni en las firmas del `UseCase`, ni en una reexportación pública desde `application`. Un tipo de modelo que vive en el kernel también cuenta como dominio. `execute` recibe un solo argumento, el `Command` o la `Query` del caso de uso (la identidad del llamador va dentro), y devuelve su `Response` y su `Error`, definidos junto al handler. Sin árbol A, devuelve solo la falla excepcional, y el caso de uso no tiene tipo de error (`ERR-1`). Las conversiones (`From<&Agregado> for Response`) y los `use` del handler no son contrato. Que `api` no declare `domain` no basta: un valor de dominio que llega por un campo público se puede usar sin importar el crate. Los workflows viven solo en `workflows/`, no implementan un `UseCase` y `api` no los nombra. Un puerto entrante es un `UseCase` con solo `execute`, o un `EventSubscriber` con solo `on`. Sus implementaciones se llaman `*Subscriber`, viven en `subscribers/` y no implementan un `UseCase`. En Rust: `tests/naming.rs::hex_4_*`.

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
| `Publisher` | Despachar eventos de dominio dentro del proceso, a los suscriptores del contexto (HEX-7) |
| `Service` | Utilidad técnica que no encaja en lo anterior (último recurso). Es el único uso del sufijo: nunca en `domain` ni en `application` fuera de `ports/outbound` (HEX-7) |

Los adaptadores se nombran `{Tecnología}{Concepto}Adapter`, donde **Concepto es el sustantivo del puerto sin el sufijo de rol** (`Postgres…`, `InMemory…`, `Http…`, `NoOp…`):

| Puerto | Adaptador |
|--------|-----------|
| `CommunityRepository` | `PostgresCommunityAdapter`, `InMemoryCommunityAdapter` |
| `ModerationGateway` | `HttpModerationAdapter`, `NoOpModerationAdapter` |

**Puertos con nombre propio.** Una lista cerrada de patrones conocidos se nombra sin sufijo, porque el nombre ya dice el rol. Su Concepto es el nombre completo:

| Puerto | Rol | Adaptador |
|--------|-----|-----------|
| `UnitOfWork` | Frontera transaccional: lo que corre dentro se confirma o se revierte junto | `PostgresUnitOfWorkAdapter`, `InMemoryUnitOfWorkAdapter` |

La transacción la declara `application`, porque decidir qué escrituras son atómicas es orquestación del caso de uso. Ni el handler ni el dominio conocen la tecnología que la implementa. Agregar un nombre a esta lista exige cambiarla junto con su test, en el mismo commit.

La otra excepción: las capacidades técnicas transversales del kernel (HEX-8) no llevan sufijo de rol. Sus adaptadores sí siguen esta regla, con el nombre completo de la capacidad como Concepto: `UuidIdGeneratorAdapter`, `SystemClockAdapter`, `StderrFaultReporterAdapter`.

- **Prefijos reservados.** `Fake`, `Fixed`, `Stub` y `Spy` son de los dobles de test (`TDD-2`). Ningún adaptador de producción los usa: un `FakeClockAdapter` en producción es un doble que se coló, o un adaptador mal nombrado.
- **Ubicación.** Los adaptadores de producción viven en el anillo de infraestructura, también los de las capacidades del kernel. Uno que no pertenece a ningún contexto va en una infraestructura compartida que solo ve el kernel, nunca en el composition root, que cablea pero no implementa.

**Fundamento.** Al leer la firma de un handler se sabe qué toca: una base de datos, un tercero, un servicio propio. `Port` es uniforme pero opaco. En el adaptador, el rol ya está implícito en la tecnología (`Postgres` es persistencia, `Http` es remoto); repetirlo solo alarga el nombre.

**Violaciones típicas.** `Service` usado como cajón de sastre. Un `Gateway` para un servicio propio. `PostgresCommunityRepositoryAdapter` (repite el rol). Una anotación transaccional del framework sobre el handler, en vez del puerto `UnitOfWork`. Un `TransactionService` o un `CommunityUnitOfWork`, que esconden el patrón detrás de otro nombre.

**Verificación.** Tests de arquitectura, uno por mitad de la regla. Los puertos de `ports/outbound` llevan un sufijo de rol, o uno de los nombres propios de la lista, y nunca `Port`. En todo el código, de producción y de test, todo tipo que implementa un puerto saliente o una capacidad del kernel se llama `{Prefijo}{Concepto}Adapter`. Así falla `PostgresCommunityRepositoryAdapter`, que repite el rol. En código de test, el prefijo es uno de los dobles (`TDD-2`). En producción, el prefijo no está vacío, no es de un doble, y el adaptador vive en infraestructura. En Rust: `tests/naming.rs::hex_5_*`.

---

## HEX-6 — Vocabulario de tipos sin colisiones

**Regla.**

- Los DTOs de un puerto saliente no se anidan en la interfaz: viven en `ports/outbound/dtos`, agrupados en un contenedor por contrato (`{Contrato}Payloads`). También su error declarado (`{Contrato}Error`, `ERR-3`), nunca una hoja de los errores de `application`: esa traducción es del handler.
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

**Verificación.** Test de arquitectura: ningún tipo en `ports/outbound/dtos` termina en `Command`, `Query`, `Response` ni `Result`; en `api`, los tipos `*Request`/`*Response` llevan el nombre del transporte. Todo tipo de `application` que aparece en la firma de un puerto saliente (un DTO, un error declarado) está definido en `ports/outbound/dtos`; el puerto puede usar además el dominio, el kernel y los tipos estándar.

---

## HEX-7 — El dominio decide; la aplicación orquesta

**Regla.**

| Capa | Sí | No |
|------|----|----|
| Dominio | Validaciones, invariantes, transiciones de estado, reglas | Puertos, framework, mapeo de transporte o persistencia |
| Aplicación | Leer el input, cargar, invocar al dominio, llamar puertos salientes, mapear la respuesta | `if` de negocio, validar campos del agregado, reglas de estado |
| Api | Forma del transporte (validación sintáctica del payload) | Duplicar reglas de dominio |

Agregados inmutables con comportamiento en sus métodos. Las reglas se evalúan en el dominio y devuelven `Result` (`ERR-1`).

**Crear y reconstituir.** Un agregado nace por una vía con nombre propio (`create`) que aplica lo que solo vale al nacer: el estado inicial, las reglas de alta y el evento inicial (`DomainEvent`). Reconstituirlo desde la persistencia es otra vía (`restore` o el constructor canónico), que acepta cualquier estado y no repite las reglas de alta ni emite eventos. La forma depende del lenguaje: funciones asociadas del agregado, o un `{Agregado}Factory` en el dominio si el constructor del lenguaje es público y no se puede reservar para la reconstitución (los `record` de Java). El identificador lo recibe de afuera (`IdGenerator`, HEX-8), no lo genera el dominio.

**Eventos de dominio.** Una transición del agregado registra su evento (`DomainEvent`, HEX-8) en el estado nuevo que devuelve: el agregado sigue siendo inmutable y el evento viaja con él. El handler orquesta la publicación:

1. Toma los eventos pendientes (`take_events`). El agregado que guarda no tiene ninguno, porque los eventos no se persisten con él.
2. Guarda y publica por un puerto saliente `{Concepto}EventPublisher` (rol `Publisher`, HEX-5), **solo si guardó**. Con un `UnitOfWork`, las dos cosas van en la misma transacción.
3. El despachador es un adaptador de infraestructura (`InProcess{Concepto}EventAdapter`), no código de `application`. Entrega cada evento, en orden, a los suscriptores (`{Concepto}EventSubscriber`, HEX-4) que le cablea el composition root, y la primera falla corta la publicación.

Reconstituir un agregado no emite eventos. Publicar hacia otro sistema no es esto: es un `Synchronizer`, que se llama después de confirmar el estado (éxito degradado, ERR-3).

**Servicio de dominio.** Una regla que no pertenece a un solo agregado (una política de contraseñas, una regla que compara dos agregados) va en un servicio de dominio: vive en `domain`, no tiene estado, recibe valores del dominio y devuelve `Result`. No recibe puertos ni hace E/S: si la regla necesita datos externos, el handler los carga y se los pasa. Se nombra por la regla que evalúa (`PasswordPolicy`, `MembershipEligibility`), **nunca `…Service`**. Un tipo que solo enumera opciones (`MembershipPolicy { Open, Closed }`) es un value object, no un servicio.

**Fundamento.** Un handler con lógica de negocio es un modelo anémico disfrazado. Si la regla vive en el agregado, se prueba sin infraestructura y no se duplica entre casos de uso.

**Violaciones típicas.** El handler calcula, valida longitudes o decide si una transición es válida. El handler arma el evento en vez de tomarlo del agregado, publica antes de guardar o persiste los eventos con el agregado. El despachador de eventos como clase de `application`. Es **la violación más frecuente de los agentes de IA**: tienden a escribir el `if` donde están editando. Un `PasswordService` en el dominio, o un servicio de dominio que recibe un repositorio.

**Verificación.** La lógica fuera del dominio y que un servicio de dominio no tenga estado ni E/S, por revisión (difícil de automatizar): el subagente `architecture-reviewer`. El nombre, por test de arquitectura: en `domain` y en `application` (fuera de `ports/outbound`) ningún tipo termina en `Service`. En Rust: `tests/naming.rs::hex_7_service_suffix_is_only_an_outbound_role`.

---

## HEX-8 — Kernel compartido delgado

**Regla.** Un módulo de kernel compartido contiene **solo primitivas**: `Result`, los tipos de los dos árboles de error (`ERR-1`), los marcadores de modelo (`ValueObject`, `Entity`, `AggregateRoot`, `Identifier`, `DomainEvent`), los marcadores de error (`DomainError`, `ApplicationError`) **solo si el lenguaje los necesita** y las **capacidades técnicas transversales** de la lista de abajo. Los únicos tipos abstractos (traits, interfaces) son esos marcadores y esas capacidades.

Un marcador de error hace falta cuando el lenguaje acota las jerarquías cerradas por módulo: en Java, un `sealed` del kernel no puede admitir tipos de otro módulo, así que cada contexto sella su `{Bc}DomainError extends DomainError`. Donde el contrato se cierra con un `enum` del propio contexto, como en Rust, el marcador no aporta nada y el kernel no lo declara. Un marcador no lleva dependencias ni ítems: no impone métodos, tipos ni constantes asociados. Puede exigir rasgos estándar (igualdad, copia) u otro marcador, nunca una capacidad. Cada tipo expone su propia API. Si un evento necesita su instante, lo recibe de `Clock` al crearse, no de una librería de fechas. Nunca agregados de un contexto, puertos de un caso de uso ni datos sensibles. Sin framework ni dependencias: solo el lenguaje estándar. Si el kernel se divide en capas, HEX-1 rige dentro de él.

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
- **Adaptadores:** siguen HEX-5. Como la capacidad no tiene sufijo de rol que quitar, el Concepto es su nombre completo: `UuidIdGeneratorAdapter`, `SystemClockAdapter`. Viven en la infraestructura compartida, fuera de los contextos: todos los contextos usan la misma capacidad, y su adaptador existe una sola vez.

**Fundamento.** Un sitio único para los bloques de construcción evita copiar contratos sin acoplar contextos entre sí. Si crece, se vuelve un acoplamiento disfrazado. Por eso las capacidades tienen criterio y lista cerrada: "varios contextos lo usan" no basta, porque un puerto de negocio compartido es justo el acoplamiento que el kernel debe evitar. Sin sufijo de rol, porque el rol de una capacidad no varía. Un `Clock` no es ni persistencia ni un tercero: es el reloj.

**Violaciones típicas.** Mover un agregado al kernel "porque dos servicios lo usan". Agregar un puerto con vocabulario de negocio (`CommunityLookup`) al kernel para compartirlo entre contextos. Una capacidad que no está en la lista. Un adaptador de capacidad sin sufijo (`UuidIdGenerator`). El adaptador de una capacidad dentro de la infraestructura de un contexto.

**Verificación.** Test de arquitectura propio del kernel: no tiene dependencias, y todo trait público es un marcador de modelo o una capacidad de la lista. Los nombres de los adaptadores los cubre el test de HEX-5. Otro test exige que los adaptadores de las capacidades vivan fuera de los contextos (los dobles de test no cuentan). Otro, que los marcadores no tengan ítems. En Rust: `tests/kernel.rs::hex_8_*`, `tests/naming.rs::hex_5_adapters_*` y `tests/naming.rs::hex_8_capability_adapters_are_shared_infrastructure`.

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
| `PasswordService` en el dominio | Servicio de dominio nombrado por la regla: `PasswordPolicy` (HEX-7) |

---

## Decisiones abiertas

Ninguna por ahora. Una decisión nueva se anota aquí hasta que tenga regla.
