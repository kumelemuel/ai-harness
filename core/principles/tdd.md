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
- El **composition root** (`HEX-2`) no lleva test unitario propio: solo cablea, y lo cubren los tests de endpoint. Cablear un handler nuevo no exige un test aparte.

**Fundamento.** El test que falla primero demuestra que el test prueba algo y que el código existe porque un comportamiento lo exigió. Con agentes de IA es además el mecanismo de control: el agente converge hacia el test, así que el test define el objetivo.

**Violaciones típicas.** Escribir implementación y test a la vez. Un test que nunca se vio fallar. Ajustar la aserción al resultado obtenido en vez de corregir el código.

**Verificación.** No automatizable de forma fiable. Asignado al subagente revisor (`architecture-reviewer`; en sitios, `site-auditor`). Revisa que todo cambio de producción del diff tenga un cambio de test que lo exija, salvo el cableado del composition root. Si hay varios commits, el test aparece antes o junto a la producción. Los hooks garantizan verde, no rojo primero.

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
- Los dobles de los puertos salientes son ciudadanos de primera clase y se mantienen como código de producción:
  - **Dónde:** viven donde los tests del núcleo pueden verlos, sin que el núcleo dependa de `infrastructure` (`HEX-1`).
  - **Nombre:** se nombran como adaptadores (`HEX-5`), `{Prefijo}{Concepto}Adapter`, con un prefijo de esta lista **cerrada**:

    | Prefijo | Doble |
    |---------|-------|
    | `Fake` | Con estado (`FakeCommunityAdapter`) |
    | `Fixed`, `Stub` | Respuesta fija (`FixedIdGeneratorAdapter`) |
    | `Spy` | Registra las llamadas para que el test las afirme (`SpyFaultReporterAdapter`) |

    Aplica a **todos** los dobles: los compartidos y los locales a un solo archivo de test. Ningún adaptador de producción usa estos prefijos (`HEX-5`).
  - **Contrato:** pasan la misma **batería de contrato** del puerto que los adaptadores reales. Si el doble diverge del adaptador real, la batería falla.
- Cada puerto saliente tiene una batería de contrato: los comportamientos de los que dependen los handlers, escritos una vez y ejecutados contra el doble y contra cada adaptador.
- Un adaptador en memoria de `infrastructure`, que sirve para correr sin base de datos, es un adaptador de producción, no un doble de test.
- Los tests de caso de uso afirman sobre `Result`, no sobre excepciones (`ERR-1`).

**Fundamento.** Cada capa se prueba en su nivel más barato. Un núcleo sin framework (`HEX-2`) es lo que permite probar casos de uso con instancias puras y fakes, sin levantar infraestructura.

**Violaciones típicas.** Levantar el contenedor del framework para probar un handler. Mocks de librería en lugar de fakes en memoria. Probar reglas de negocio a través del endpoint. Un fake que no pasa la batería del puerto: el handler queda probado contra un comportamiento que ningún adaptador real tiene.

**Verificación.** Test de arquitectura sobre las dependencias de test del núcleo: dominio y aplicación no declaran runtimes ni frameworks, tampoco como dependencias de test. En Rust, `tests/dependencies.rs::hex_2_core_*` lee las `[dev-dependencies]`, y Cargo impide usar en un test lo que no está declarado. Los nombres de los dobles los cubre el test de nombres de adaptadores (`HEX-5`). En Rust, `hex_5_adapters_*` recorre `src/` y `tests/` de todos los crates, incluidos los módulos `#[cfg(test)]` y `testing`. El test ve el prefijo, no lo que hace el doble: que un `Stub` no registre llamadas y que un `Spy` sí lo haga queda asignado al subagente revisor. Que doble y adaptadores cumplan lo mismo lo garantiza su batería de contrato. La forma de los tests queda asignada al subagente revisor: contenedor del framework, mocks de librería en vez de fakes, reglas de negocio probadas por el endpoint.

---

## Antipatrones

| En vez de | Usar |
|-----------|------|
| Código de producción sin test | TDD-1 |
| Test escrito después y nunca visto fallar | Rojo primero |
| Contenedor del framework en tests de núcleo | `new Handler(fakePort)` |
| Mocks de librería para puertos salientes | Fakes en memoria |
| Fake sin test de contrato | Batería de contrato compartida |
