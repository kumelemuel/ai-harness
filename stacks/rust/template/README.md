# Plantilla Rust — craft-backend

Workspace mínimo que materializa [`stacks/rust`](../README.md): un contexto
(`communities`) con un caso de uso (`create_community`) de punta a punta.

```sh
cargo test --workspace                                  # 35 tests, incluidos los de arquitectura
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p server                                     # BIND_ADDRESS=127.0.0.1:3000 por defecto

curl -X POST localhost:3000/communities \
  -H 'content-type: application/json' -H 'x-account-id: alice' \
  -d '{"slug":"rust-chile","name":"Rust Chile","is_public":true}'
```

## Usarla en un proyecto nuevo

1. Copia este directorio como raíz del proyecto.
2. Renombra `communities` y su caso de uso, o úsalos como guía y bórralos.
3. Para un contexto nuevo: crea `contexts/{ctx}/{domain,application,infrastructure,api}`,
   agrégalos a `[workspace.dependencies]`, repite la forma de `communities` y compón
   su `{Ctx}Api` en `apps/server`.
4. `tools/architecture` no se toca: descubre los crates por ubicación.
