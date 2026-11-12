# Principios — Manejo de errores

> **Estado:** v0.2
> **Origen:** destilado de un proyecto Java 21 + Spring Boot con 7 ADR. Pendiente de validar en Kotlin, Rust y TypeScript.
> **Relacionados:** [hexagonal.md](hexagonal.md) (`HEX`), [verification.md](verification.md) (`VER`).

Formato de cada principio: ver [README](README.md).

---

## ERR-1 — Dos árboles de errores que no comparten jerarquía

**Regla.**

- **Árbol A — esperado.** El caso de uso puede y debe ramificar: el colaborador no completó, o el input viola una regla. Se **devuelve** dentro de `Result<T, E>`.
- **Árbol B — excepcional.** No hay interpretación de negocio razonable en ese punto: infraestructura no declarada, invariante interno roto, error de programación. Se **propaga** y se resuelve en el borde.
- Un error de negocio y una señal excepcional **nunca** comparten tipo base.
- `Result` es propio o nativo del lenguaje, con API mínima: `ok`, `err`, `map`, `flatMap`, `mapError`, `fold`. **Sin `getOrThrow`**: reintroduce el árbol B por la puerta trasera.
- Búsquedas `find*` devuelven opcional; el handler convierte "vacío" en `NotFound` solo cuando el caso de uso exige existencia.
- Si un caso de uso no tiene árbol A, no tiene tipo de error.

**Fundamento.** La firma es el contrato: quien llama no puede olvidar un fallo de negocio. Mezclar los dos árboles obliga a capturar lo que no se puede manejar o permite ignorar lo que sí.

**Violaciones típicas.** Lanzar excepciones genéricas para reglas de negocio. Un `catch` amplio que convierte fallos de infraestructura en errores de negocio.

**Verificación.** Test de arquitectura: tipos de error del árbol A no extienden la jerarquía de excepciones. Tests de caso de uso afirman sobre `Result`, no sobre excepciones.

---

## ERR-2 — Contrato de error acotado por caso de uso

**Regla.**

- Cada caso de uso con árbol A tiene su propio tipo cerrado `{Verb}{Noun}Error`, junto a su handler en `application/commands/{caso}` o `application/queries/{caso}`. **Solo admite las variantes que ese caso de uso puede producir.**
- Las variantes (hojas) se comparten dentro del bounded context (`Validation`, `NotFound`, `Conflict`, `DependencyUnavailable`…) y viven en `application/errors`, ya que `api` solo ve `application` (`HEX-1`). Una hoja declara a qué contratos pertenece.
- No existe un error global del contexto en la firma de los casos de uso.
- El handler traduce errores de dominio y de puertos a **una hoja plana** que su contrato admite. No se envuelven (`FromDomain`, `FromVault`).
- La semántica de un mismo fallo **la decide el producto, por caso de uso**: el mismo rechazo puede ser un estado de éxito en un caso de uso y un error en otro.

**Fundamento.** Exhaustividad real: el `match` de cada caso de uso no incluye variantes imposibles. Envolver reabre el catálogo del dominio o del puerto en la frontera. Hojas compartidas evitan duplicar tipos sin perder el acotamiento.

**Violaciones típicas.** Un `AppError` con veinte variantes en todas las firmas. Asumir que un fallo significa lo mismo en todos los flujos. Exponer un error de dominio directamente en el contrato del caso de uso.

**Verificación.** Compilador (exhaustividad) + tests por variante admitida.

---

## ERR-3 — Semántica de adaptadores: "respondió" frente a "no pudo completar"

**Regla.**

- Si el colaborador **respondió** (aprobó, rechazó, evaluó), eso es **éxito** del puerto, aunque la respuesta sea negativa para el negocio. El rechazo viaja en el DTO.
- Si el colaborador **no pudo completar** (no encontrado, timeout, caída), eso es un error **declarado** en la firma del puerto (`{Contrato}Error`).
- El adaptador cumple la firma: lo declarado → `err`; cualquier otra falla técnica → árbol B. **El adaptador no decide qué ramificará el caso de uso.**

**Fundamento.** Separa la respuesta de negocio de la disponibilidad técnica y mantiene la decisión de negocio en el núcleo.

**Violaciones típicas.** Un adaptador que convierte un rechazo de negocio en excepción. Un adaptador que "ayuda" al caso de uso devolviendo un error ya pensado para HTTP.

**Verificación.** Tests de adaptador por cada variante declarada.

---

## ERR-4 — El transporte se traduce solo en el borde

**Regla.**

- Códigos HTTP (o de cualquier transporte) existen **solo** en `api`. El núcleo no conoce `HttpStatus`.
- El adaptador *driving* hace `fold` exhaustivo del error del caso de uso hacia el formato de transporte (para HTTP: RFC 9457 *Problem Details*), con helpers por hoja.
- El árbol B termina en un manejador global: respuesta genérica sin detalles internos, y registro a través de un puerto saliente.
- El núcleo no hace logging. Ningún error, mensaje ni registro lleva datos sensibles ni secretos.

**Fundamento.** El núcleo sirve a cualquier transporte. Un manejador global es la red de seguridad del árbol B, no el mecanismo del árbol A.

**Violaciones típicas.** Un `if (error) return 404` dentro del handler. Mensajes de excepción con el payload del request.

**Verificación.** Test de arquitectura: el núcleo no importa tipos de transporte ni de logging.

---

## Antipatrones

| En vez de | Usar |
|-----------|------|
| Excepciones para fallos de negocio | `Result` (árbol A) |
| `getOrThrow` | `fold` / `match` exhaustivo |
| Error global del contexto en las firmas | `{Verb}{Noun}Error` acotado |
| Errores envueltos (`FromDomain`) | Hoja plana del contrato |
| Adaptador que lanza ante un rechazo de negocio | Rechazo como éxito del puerto, en el DTO |
| Código de transporte en el núcleo | Traducción en `api` |
| Logging en el núcleo | Registro vía puerto saliente desde el manejador global |

---

## Decisiones por stack

- **Árbol B en Rust:** resuelto en [`stacks/rust`](../../stacks/rust/README.md) (RS-2): `Fault` opaco propagado con `?` dentro de `Failure<E>`, y `panic!` solo para invariantes rotos.
