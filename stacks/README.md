# Stacks

Traducción de los principios de [`core/principles`](../core/principles/README.md)
a cada lenguaje. Cada stack tiene su propio directorio con el detalle; esta
tabla es el índice.

| Stack | Estado |
|-------|--------|
| [Rust](rust/README.md) | v0.2, con [plantilla ejecutable](rust/template/) |
| [Astro](astro/README.md) (sitios web) | v0.1, con [plantilla ejecutable](astro/template/) |
| Java, Kotlin, TypeScript | Pendientes |

| Concepto | Java 21 | Kotlin | Rust | TypeScript |
|----------|---------|--------|------|------------|
| `Result` (`ERR-1`) | Tipo propio sellado en el kernel | Tipo propio `sealed` en el kernel | `Result<T, E>` nativo | Unión discriminada propia |
| Tipo de error cerrado (`ERR-2`) | `sealed interface` + `permits` | `sealed interface` | `enum` | Unión discriminada |
| Árbol B (`ERR-1`) | `RuntimeException` propias | Excepciones propias | `Fault` opaco en `Failure<E>`; `panic!` solo para invariantes | `Error` propias |
| Composition root (`HEX-2`) | Configuración del contenedor | Cableado manual o contenedor ligero | `main` / módulo de wiring | Módulo de wiring |
| Enforcement de capas (`VER-1`) | ArchUnit | ArchUnit o Konsist | Crate por capa + tests `toml`/`syn` | dependency-cruiser |
| Puertos async | — | `suspend fun` | `BoxFuture` del kernel | `Promise` |
| `ports/inbound`, `ports/outbound` | `ports.inbound` | `ports.inbound` | `ports::inbound` | `ports/inbound` |
