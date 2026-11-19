# Plantilla Rust — craft-backend

Workspace mínimo que materializa [`stacks/rust`](../README.md), con dos contextos:

- `communities`: un comando (`create_community`) de punta a punta.
- `iam`: una query (`verify_access_token`) y el middleware que protege las rutas de
  los demás contextos con un JWT HS256.

```sh
cargo test --workspace                                  # todos en verde, incluidos los de arquitectura
cargo clippy --workspace --all-targets -- -D warnings
JWT_SECRET=dev-secret cargo run -p server               # obligatorio; BIND_ADDRESS=127.0.0.1:3000 por defecto
```

La plantilla no emite tokens (no hay inicio de sesión). Para probar, genera uno HS256
con `sub` y `exp` y la misma clave:

```sh
TOKEN=$(python3 -c 'import base64,hmac,hashlib,json,time
b=lambda d: base64.urlsafe_b64encode(d).rstrip(b"=").decode()
m=b(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}")+"."+b(json.dumps({"sub":"alice","exp":int(time.time())+3600}).encode())
print(m+"."+b(hmac.new(b"dev-secret",m.encode(),hashlib.sha256).digest()))')

curl -X POST localhost:3000/communities \
  -H 'content-type: application/json' -H "authorization: Bearer $TOKEN" \
  -d '{"slug":"rust-chile","name":"Rust Chile","is_public":true}'
```

## Usarla en un proyecto nuevo

1. Copia este directorio como raíz del proyecto.
2. Renombra `communities` y su caso de uso, o úsalos como guía y bórralos.
3. Para un contexto nuevo: crea `contexts/{ctx}/{domain,application,infrastructure,api}`,
   agrégalos a `[workspace.dependencies]`, repite la forma de `communities`, construye
   su `{Ctx}Api` en `apps/server/src/contexts/{ctx}.rs` y compón sus rutas en `main.rs`.
4. `tools/architecture` no se toca: descubre los crates por ubicación.
