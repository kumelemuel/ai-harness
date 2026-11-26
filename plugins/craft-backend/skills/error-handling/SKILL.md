---
name: error-handling
description: Reglas de manejo de errores para backends con capas domain/application/api/infrastructure, con Result y dos árboles de errores (negocio frente a excepcional). Usar SIEMPRE, en un backend de ese tipo, antes de crear o modificar tipos de error, firmas que devuelven Result, adaptadores que llaman a terceros o persistencia, manejadores globales de excepciones o la traducción de errores a HTTP/Problem Details.
---

# Manejo de errores

Fuente completa con fundamentos: `references/errors.md` (principios `ERR-1` a `ERR-4`).
Léela cuando una regla de abajo no alcance para decidir. **No te desvíes sin
citar el ID del principio y explicar por qué.**

## Decidir el árbol (ERR-1)

Ante cualquier fallo, pregunta: *¿el caso de uso puede ramificar de forma
razonable con esto?*

- **Sí → árbol A.** Se **devuelve** en `Result<T, E>`. Ejemplos: regla de negocio
  violada, recurso requerido inexistente, colaborador que declaró no poder
  completar.
- **No → árbol B.** Se **propaga** hasta el manejador global. Ejemplos: falla de
  infraestructura no declarada, invariante interno roto, bug.

Reglas:
- Los dos árboles **nunca** comparten tipo base.
- `Result` con API mínima: `ok`, `err`, `map`, `flatMap`, `mapError`, `fold`.
  **Nunca `getOrThrow`.**
- `find*` devuelve opcional; el handler lo convierte en `NotFound` solo si el
  caso de uso exige existencia.
- Un caso de uso sin árbol A no tiene tipo de error.

## Contrato de error (ERR-2)

- Cada caso de uso con árbol A tiene su `{Verb}{Noun}Error` cerrado, junto a su
  handler. Contiene **solo** las variantes que ese caso de uso puede producir.
- Las hojas (`Validation`, `NotFound`, `Conflict`, `DependencyUnavailable`…) se
  comparten dentro del contexto en `application/errors`.
- El campo que nombra una hoja es un enum cerrado del contexto (`Field`), nunca
  un texto. Un campo con varios motivos de rechazo, cada uno con su código,
  lleva el motivo en su variante (`Field::Owner(OwnerRejection)`); no es una
  hoja nueva.
- Sin error global del contexto (`AppError`) en las firmas.
- El handler traduce errores de dominio y de puertos a **una hoja plana**. No los
  envuelve (`FromDomain`, `FromVault`).
- El significado de un fallo lo decide el producto **por caso de uso**: el mismo
  rechazo puede ser éxito en un caso y error en otro.
- **Corte:** si el caso de uso persiste un estado de negocio (rechazado, fallido)
  y responde con éxito, **no es error**: va en el `Response`. Es hoja del contrato
  solo si aborta (sin estado nuevo, o revirtiendo) y el cliente debe corregir o
  reintentar.

## Adaptadores (ERR-3)

- El colaborador **respondió** (aprobó, rechazó, evaluó): es **éxito** del
  puerto. El rechazo viaja en el DTO. Ejemplo: el servicio de moderación
  rechaza el nombre de una comunidad → `ok` con un `ModerationVerdict` de
  rechazo (`Rejected`, con su motivo); `CreateCommunity` lo traduce a la hoja
  `ContentRejected`.
- El colaborador **no pudo completar** (no encontrado, timeout, caída): es un
  error **declarado** en la firma del puerto (`{Contrato}Error`). Ejemplo:
  la moderación no responde a tiempo → `ModerationError.Unavailable`, que
  `CreateCommunity` traduce a la hoja `DependencyUnavailable`.
- Cualquier otra falla técnica: árbol B.
- El adaptador no decide la rama de negocio ni devuelve errores pensados
  para HTTP.
- **Éxito degradado:** un colaborador llamado **después** de confirmar el estado
  (sincronizar, notificar) no revierte el caso de uso si falla. Su error
  declarado queda registrado en el estado (`synced = false`), sin hoja ni `Fault`.

## Borde (ERR-4)

- Códigos de transporte solo en `api`. El núcleo no conoce `HttpStatus`.
- `api` hace `fold` exhaustivo del error del caso de uso hacia RFC 9457
  *Problem Details*, con un helper por hoja. Sin brazo comodín (`_`, una
  variable, `Err(_)`) sobre el contrato ni sobre `Field`: una variante nueva
  tiene que no compilar, no caer en un 400 genérico.
- Árbol B: manejador global, respuesta genérica sin detalles internos y registro
  a través de un puerto saliente.
- El núcleo no hace logging. Nunca datos sensibles ni secretos en errores,
  mensajes o registros.
- Registro de un resultado de negocio que el producto necesita: un puerto
  saliente (`{Concepto}LogService`) que invoca el handler. El `FaultReporter` solo
  lo invoca el manejador global (árbol B).

## Tests

- Los tests de caso de uso afirman sobre `Result`, no sobre excepciones.
- Hay un test por cada variante admitida del contrato de error.
- La batería de contrato de un puerto con errores declarados tiene un caso por
  resultado (cada respuesta y cada variante declarada), provocado por un
  escenario de la fábrica. Un adaptador que no puede producir un escenario lo
  declara con su motivo; nunca se lo salta en silencio.
