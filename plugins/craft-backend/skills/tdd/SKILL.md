---
name: tdd
description: Disciplina TDD (rojo → verde → refactor) y estrategia de tests por capa. Usar SIEMPRE antes de escribir o modificar código de producción, al agregar un caso de uso, agregado o adaptador, y cuando un test falla y hay tentación de cambiar el test.
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

- Los puertos salientes se sustituyen con **fakes en memoria** (`InMemory…Adapter`),
  no con mocks de librería. Los fakes son código de primera clase.
- Los tests de dominio y de caso de uso no levantan el framework.
- Los tests de caso de uso afirman sobre `Result`, no sobre excepciones.
