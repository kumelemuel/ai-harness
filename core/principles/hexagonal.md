# Principios — Arquitectura hexagonal

> **Estado:** v0.2
> **Origen:** destilado de un proyecto Java 21 + Spring Boot con 7 ADR. Pendiente de validar en Kotlin, Rust y TypeScript.
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

**Fundamento.** La aplicación es la frontera lógica del caso de uso: declara qué ofrece y qué necesita. Persistencia e integraciones son detalle de orquestación, no reglas de negocio. Un dominio sin puertos no cambia cuando cambia la frontera técnica, y no conoce quién lo invoca.

> **Nota de precisión teórica.** Esto diverge conscientemente de Evans (*DDD*, 2003), que ubica las interfaces de `Repository` en la capa de dominio. Es coherente con Cockburn, donde los puertos pertenecen al borde de la *aplicación*. La divergencia debe quedar explícita para que nadie la "corrija" citando a Evans.

**Violaciones típicas.** Crear `domain/ports` por costumbre. Un agregado que recibe un repositorio para cargarse a sí mismo.

**Verificación.** Test de arquitectura: ningún tipo de `domain` es una interfaz de puerto; `domain` no depende de `application`.

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

**Violaciones típicas.** Un `PaymentService` con diez métodos. Un controller que inyecta el handler. Un `Response` que expone el agregado tal cual.

**Verificación.** Test de arquitectura: `api` no depende de `*Handler`; cada `*UseCase` declara un solo método; los tipos de `commands/` y `queries/` usados en firmas de `UseCase` no referencian `domain`.

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
| `PaymentRepository` | `PostgresPaymentAdapter`, `InMemoryPaymentAdapter` |
| `FraudGateway` | `HttpFraudAdapter`, `StubFraudAdapter` |

**Fundamento.** Al leer la firma de un handler se sabe qué toca: una base de datos, un tercero, un servicio propio. `Port` es uniforme pero opaco. En el adaptador, el rol ya está implícito en la tecnología (`Postgres` es persistencia, `Http` es remoto); repetirlo solo alarga el nombre.

**Violaciones típicas.** `Service` usado como cajón de sastre. Un `Gateway` para un servicio propio. `PostgresPaymentRepositoryAdapter` (repite el rol).

**Verificación.** Test de arquitectura sobre nombres de interfaces en `ports/outbound`.

---

## HEX-6 — Vocabulario de tipos sin colisiones

**Regla.**

- Los DTOs de un puerto saliente no se anidan en la interfaz: viven en `ports/outbound/dtos`, agrupados en un contenedor por contrato (`{Contrato}Payloads`).
- Cada frontera tiene su vocabulario, y ningún sufijo se repite entre fronteras:

| Frontera | Entrada | Salida | Dónde |
|----------|---------|--------|-------|
| Caso de uso | `Command` / `Query` | `Response` | `application/commands`, `application/queries` |
| Puerto saliente | `Request` | Nombre del concepto, sin sufijo (`Authorization`, `DetokenizedCard`) | `application/ports/outbound/dtos` |
| Transporte | `{Caso}{Transporte}Request` | `{Caso}{Transporte}Response` | `api` |
| Control de flujo | — | `Result` (`ERR-1`) | kernel |

- Para HTTP: `{Caso}HttpRequest` y `{Caso}HttpResponse`. Otros transportes siguen el patrón (`{Caso}GrpcRequest`, `{Caso}Message`).
- Un puerto cuyas firmas solo usan tipos de dominio no gana un contenedor de payloads vacío.

**Fundamento.** La interfaz queda como lista de operaciones. Un vocabulario sin colisiones hace que el nombre de un tipo diga a qué frontera pertenece. Los tipos de transporte llevan el nombre del transporte porque son los únicos que cambian si cambia el transporte.

**Violaciones típicas.** `ChargeCommand` como DTO de un gateway. `AuthorizationResult` como payload de éxito. Un controller que recibe un `ChargeRequest` (colisiona con el vocabulario del puerto saliente) o devuelve el `Response` del caso de uso tal cual.

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

**Regla.** Un módulo de kernel compartido contiene **solo primitivas**: `Result`, marcadores de modelo (`ValueObject`, `Entity`, `AggregateRoot`, `Identifier`), marcadores de error y de evento, y helpers genéricos (`Clock`). Nunca agregados de un contexto, puertos de un caso de uso ni datos sensibles. Sin framework. Dentro del kernel, `domain` no depende de `application`.

Las jerarquías cerradas de cada contexto viven en el contexto, no en el kernel (los marcadores del kernel son abiertos).

**Fundamento.** Un sitio único para los bloques de construcción evita copiar contratos sin acoplar contextos entre sí. Si crece, se vuelve un acoplamiento disfrazado.

**Violaciones típicas.** Mover un agregado al kernel "porque dos servicios lo usan".

**Verificación.** Test de arquitectura propio del kernel.

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
| Reglas de negocio en el handler | Encapsularlas en el dominio |

---

## Decisiones abiertas

1. **Criterio para el sufijo `Service`.** Definir cuándo se permite, para que no se vuelva cajón de sastre.
