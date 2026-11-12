---
name: error-handling
description: Reglas de manejo de errores para backends con Result y dos árboles de errores (negocio frente a excepcional). Usar SIEMPRE antes de crear o modificar tipos de error, firmas que devuelven Result, adaptadores que llaman a terceros o persistencia, manejadores globales de excepciones o la traducción de errores a HTTP/Problem Details.
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
- Sin error global del contexto (`AppError`) en las firmas.
- El handler traduce errores de dominio y de puertos a **una hoja plana**. No los
  envuelve (`FromDomain`, `FromVault`).
- El significado de un fallo lo decide el producto **por caso de uso**: el mismo
  rechazo puede ser éxito en un caso y error en otro.

## Adaptadores (ERR-3)

- El colaborador **respondió** (aprobó, rechazó, evaluó): es **éxito** del
  puerto. El rechazo viaja en el DTO.
- El colaborador **no pudo completar** (no encontrado, timeout, caída): es un
  error **declarado** en la firma del puerto (`{Contrato}Error`).
- Cualquier otra falla técnica: árbol B.
- El adaptador no decide la rama de negocio ni devuelve errores pensados
  para HTTP.

## Borde (ERR-4)

- Códigos de transporte solo en `api`. El núcleo no conoce `HttpStatus`.
- `api` hace `fold` exhaustivo del error del caso de uso hacia RFC 9457
  *Problem Details*, con un helper por hoja.
- Árbol B: manejador global, respuesta genérica sin detalles internos y registro
  a través de un puerto saliente.
- El núcleo no hace logging. Nunca datos sensibles ni secretos en errores,
  mensajes o registros.

## Tests

- Los tests de caso de uso afirman sobre `Result`, no sobre excepciones.
- Hay un test por cada variante admitida del contrato de error, y uno por cada
  variante declarada en cada adaptador.
