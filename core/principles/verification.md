# Principios — Verificación automática

> **Estado:** v0.2
> **Relacionados:** todos los principios con sección **Verificación**.

Formato de cada principio: ver [README](README.md).

---

## VER-1 — Lo que no se verifica se erosiona

**Regla.** Toda regla de dependencias o de nombres que pueda expresarse como test, **se expresa como test** y falla el build. La documentación explica; el test garantiza.

**Fundamento.** Sin enforcement automático, las reglas dependen de la revisión humana y se degradan. Con agentes de IA esto es todavía más cierto: un agente obedece al test que falla, no al párrafo que no leyó.

**Violaciones típicas.** Una regla documentada sin test que la respalde. Un test de arquitectura que existe pero no corre en CI. Desactivar una regla del test "temporalmente".

**Verificación.** Existe un test de arquitectura por módulo y corre en CI. Las reglas que no son automatizables (p. ej. `HEX-7`) quedan explícitamente asignadas a un revisor.

---

## Decisiones abiertas

1. **Límites de capa por módulo de build.** En el proyecto origen se descartó por costo en Gradle. En un workspace de Rust el costo es bajo y la garantía es de compilación: probablemente conviene adoptarlo ahí.
