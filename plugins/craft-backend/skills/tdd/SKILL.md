---
name: tdd
description: Disciplina TDD (rojo → verde → refactor) y estrategia de tests por capa (dominio, handler con fakes de puertos, adaptador, transporte) para backends con capas domain/application/api/infrastructure. Usar SIEMPRE antes de escribir o modificar código de producción en un backend de ese tipo, al agregar un caso de uso, agregado o adaptador, y cuando un test de ese backend falla y hay tentación de cambiar el test. No aplica a sitios web informativos: ahí rige `web-principles`.
---

# TDD

Fuente completa con fundamentos: `references/tdd.md` (principios `TDD-1` y `TDD-2`).

## El ciclo (TDD-1)

1. **Rojo.** Escribe un test para el siguiente comportamiento. Córrelo y
   **confirma que falla por la razón esperada** (no por un error de compilación
   ajeno ni por un import faltante). Muestra la salida del fallo.
2. **Verde.** Escribe la implementación mínima que lo hace pasar. Córrelo.
3. **Refactor.** Con todo en verde, mejora el diseño sin cambiar comportamiento.
   Vuelve a correr los tests.
4. Repite con el siguiente comportamiento. Un comportamiento por ciclo.

Prohibido:
- Escribir producción antes de ver el test en rojo.
- Escribir implementación y test en el mismo paso.
- **Modificar un test para que pase.** Si crees que el test está mal, detente,
  explica por qué y espera confirmación antes de cambiarlo.
- Si el gate de Stop te bloquea en rojo intencional o con un test que crees
  incorrecto, no lo modifiques: explica el estado y termina.

**Firma que pasa a ser falible** (un puerto empieza a devolver `Result` o
`Fault`): el commit de la firma descarta el error de forma explícita (`let _ =
…` con un comentario, nunca en silencio) y el `?` llega en un commit posterior,
con el test de falla en rojo contra el commit padre. Con el `?` en el commit de
la firma, el historial muestra la producción antes que el test.

## Por dónde empezar (TDD-2)

Un caso de uso nuevo empieza por el test de dominio o del handler, nunca por el
endpoint ni por la persistencia.

| Qué | Cómo |
|-----|------|
| Dominio | Test unitario puro sobre agregado o factory |
| Caso de uso | `new Handler(fakePort)`, sin contenedor |
| Adaptador | Contra su tecnología (o un sustituto fiel) |
| Transporte | Test del endpoint |
| Arquitectura | Reglas de capas y nombres |

- Los puertos salientes se sustituyen con **dobles**, no con mocks de librería.
  Los dobles son código de primera clase:
  - Viven donde los tests del núcleo pueden verlos, nunca en `infrastructure`.
  - Se nombran como adaptadores, `{Prefijo}{Concepto}Adapter`, con un prefijo
    de la lista cerrada: `Fake` (con estado), `Fixed` o `Stub` (respuesta
    fija), `Spy` (registra llamadas). `FakeCommunityAdapter`,
    `SpyFaultReporterAdapter`. **Todos** los dobles, también los locales a un
    solo archivo de test.
  - Dobles de puertos **entrantes** (`UseCase`, `EventSubscriber`), como los de
    los tests de `api`: prefijo más el nombre completo del puerto, sin `Adapter`.
    `StubCreateCommunityUseCase`, `SpyVerifyAccessTokenUseCase`. Si registra lo
    que recibe y el test lo afirma, es `Spy`, no `Stub`. Uno que no se invoca
    también es `Stub` (nunca `Unused…`).
  - Ningún adaptador de producción usa esos prefijos.
  - Pasan la **batería de contrato** del puerto, la misma que sus adaptadores
    reales. Un puerto nuevo nace con su batería. Si el puerto declara errores,
    la batería incluye un caso por variante.
  - Si el puerto puede fallar, el doble tiene `failing()` (todas las
    operaciones) y `failing_on(operación)` (solo esa; las demás cumplen el
    contrato).
- **Una prueba de falla por cada llamada a un puerto que puede fallar**:
  `fails_with_unexpected_when_<operación>_fails`, con `failing_on`. Con
  `failing()` solo se prueba la primera llamada.
- El adaptador en memoria de `infrastructure` es de producción, no un doble.
- Los tests de dominio y de caso de uso no levantan el framework.
- Los tests de caso de uso afirman sobre `Result`, no sobre excepciones.
