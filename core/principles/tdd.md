# Principios — TDD

> **Estado:** v0.2
> **Origen:** destilado de un proyecto Java 21 + Spring Boot con 7 ADR. Validado en Rust ([stacks/rust](../../stacks/rust/README.md): plantilla y migración de `communities` en kumenity-backend-rust). Pendiente de validar en Kotlin y TypeScript.
> **Relacionados:** [hexagonal.md](hexagonal.md) (`HEX`), [verification.md](verification.md) (`VER`).

Formato de cada principio: ver [README](README.md).

---

## TDD-1 — Ningún código de producción sin un test que lo exija

**Regla.**

- Ciclo rojo → verde → refactor.
- Rojo: escribe el test y **verifica que falla por la razón esperada** antes de tocar producción.
- Verde: la implementación mínima que lo hace pasar.
- Refactor: con los tests en verde, sin cambiar comportamiento.
- Nunca modifiques un test para que pase sin justificarlo explícitamente.

**Fundamento.** El test que falla primero demuestra que el test prueba algo y que el código existe porque un comportamiento lo exigió. Con agentes de IA es además el mecanismo de control: el agente converge hacia el test, así que el test define el objetivo.

**Violaciones típicas.** Escribir implementación y test a la vez. Un test que nunca se vio fallar. Ajustar la aserción al resultado obtenido en vez de corregir el código.

**Verificación.** Revisión del orden de commits; hook o skill que exige un test fallando antes de editar producción.

---

## TDD-2 — Estrategia de tests por capa

**Regla.**

| Qué | Cómo |
|-----|------|
| Dominio | Test unitario puro sobre agregado o factory |
| Caso de uso | `new Handler(fakePort)`, sin contenedor |
| Adaptador | Test del adaptador contra su tecnología (o un sustituto fiel) |
| Transporte | Test del endpoint |
| Arquitectura | Test de reglas de capas y nombres (`VER-1`) |

- Un caso de uso nuevo empieza por el test de dominio o del handler.
- Los fakes en memoria de los puertos salientes son ciudadanos de primera clase: viven junto a los tests y se mantienen como código de producción.
- Los tests de caso de uso afirman sobre `Result`, no sobre excepciones (`ERR-1`).

**Fundamento.** Cada capa se prueba en su nivel más barato. Un núcleo sin framework (`HEX-2`) es lo que permite probar casos de uso con instancias puras y fakes, sin levantar infraestructura.

**Violaciones típicas.** Levantar el contenedor del framework para probar un handler. Mocks de librería en lugar de fakes en memoria. Probar reglas de negocio a través del endpoint.

**Verificación.** Revisión; los tests de dominio y de caso de uso no importan el framework (test de arquitectura sobre el código de test).

---

## Antipatrones

| En vez de | Usar |
|-----------|------|
| Código de producción sin test | TDD-1 |
| Test escrito después y nunca visto fallar | Rojo primero |
| Contenedor del framework en tests de núcleo | `new Handler(fakePort)` |
| Mocks de librería para puertos salientes | Fakes en memoria |
