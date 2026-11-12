---
name: hexagonal-architecture
description: Reglas de arquitectura hexagonal (puertos y adaptadores) para backends. Usar SIEMPRE antes de crear o modificar casos de uso, handlers, puertos, adaptadores, agregados o el composition root en un proyecto con capas domain/application/api/infrastructure, y al revisar código de ese tipo de proyecto.
---

# Arquitectura hexagonal

Fuentes completas con fundamentos:
- `references/hexagonal.md` — principios `HEX-1` a `HEX-8`.
- `references/verification.md` — `VER-1`.

Léelas cuando una regla de abajo no alcance para decidir, o cuando vayas a
desviarte de una: **no te desvíes sin citar el ID del principio y explicar por qué**.

Skills relacionadas: `error-handling` (todo lo que sea `Result`, tipos de error,
adaptadores que fallan, traducción a HTTP) y `tdd` (cómo y en qué orden testear).

## Disposición

```
{contexto}/
├── domain/                        Modelos y reglas. Sin puertos.
├── application/
│   ├── ports/
│   │   ├── inbound/               {Verb}{Noun}UseCase
│   │   └── outbound/              {Concepto}{Rol}
│   │       └── dtos/              {Contrato}Payloads
│   ├── commands/{caso}/           Command, Response, Error, Handler
│   ├── queries/{caso}/            Query, Response, Error, Handler
│   └── errors/                    Hojas de error compartidas
├── api/                           Driving. Solo ve application.
├── infrastructure/                Driven.
└── bootstrap/                     Composition root.
```

## Reglas operativas

**Dependencias (HEX-1, HEX-2, HEX-3)**
- `domain` → `application` → `api` | `infrastructure`. Solo hacia adentro.
- `api` ve **solo** `application`: nunca importa `domain` ni `infrastructure`.
- `infrastructure` puede ver `application` y `domain`; nunca `api`.
- `domain` y `application`: solo lenguaje estándar + kernel. Sin anotaciones de
  framework ni librerías utilitarias.
- Los puertos viven en `application/ports`, **nunca** en `domain`. Diverge de
  Evans a propósito (sigue a Cockburn); no lo "corrijas".
- El cableado ocurre solo en el composition root, que expone el contrato y no
  la implementación.

**Casos de uso (HEX-4)**
- Un caso de uso = `{Verb}{Noun}UseCase` en `ports/inbound`, con un único `execute`.
- Implementación `{Verb}{Noun}Handler` en `commands/{caso}` o `queries/{caso}`,
  objeto plano con inyección por constructor.
- `Command`/`Query`, `Response` y `Error` junto al handler, como tipos de
  primer nivel. **El contrato no expone tipos de `domain`.**
- El adaptador *driving* inyecta el `UseCase`, nunca el `Handler`.

**Puertos salientes (HEX-5)**
- Sufijo por rol: `Repository`, `Cache`, `Gateway` (tercero), `Client` (servicio
  propio), `Synchronizer`, `Service` (último recurso). **Prohibido `Port`.**
- Adaptadores: `{Tecnología}{Concepto}Adapter`, con el Concepto **sin** el sufijo
  de rol: `PaymentRepository` → `PostgresPaymentAdapter`, `InMemoryPaymentAdapter`.
  Nunca `PostgresPaymentRepositoryAdapter`.

**Vocabulario sin colisiones (HEX-6)**

| Frontera | Entrada | Salida |
|----------|---------|--------|
| Caso de uso | `Command` / `Query` | `Response` |
| Puerto saliente | `Request` | Nombre del concepto (`Authorization`) |
| Transporte (`api`) | `{Caso}HttpRequest` | `{Caso}HttpResponse` |

- DTOs de puertos salientes en `ports/outbound/dtos/{Contrato}Payloads`, nunca
  anidados en la interfaz.
- Nunca `Command`, `Query`, `Response` ni `Result` como DTO de puerto saliente.

**Responsabilidades (HEX-7)**: la regla que más rompen los agentes
- El handler solo lee el input, carga, invoca al dominio, llama puertos y mapea.
- Si estás por escribir un `if` de negocio, una validación de campo o una
  transición de estado en el handler: **detente** y muévelo a un método del
  agregado que devuelva `Result`.
- Agregados inmutables, con comportamiento.

**Kernel compartido (HEX-8)**
- Solo primitivas: `Result`, marcadores (`ValueObject`, `Entity`,
  `AggregateRoot`, `Identifier`), marcadores de error y de evento, `Clock`.
- Nunca agregados, puertos de casos de uso ni datos sensibles.

**Verificación (VER-1)**
- Si el proyecto tiene tests de arquitectura, córrelos al terminar.
- Si una regla de arriba es expresable como test y no lo tiene, propónlo.

## Al terminar un cambio

Revisa el diff contra esta lista. Para cambios no triviales en `application` o
`domain`, delega la revisión al subagente `architecture-reviewer`.
