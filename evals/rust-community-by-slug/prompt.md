Agrega una consulta para obtener una comunidad por su slug.

Contrato HTTP:

- `GET /communities/{slug}`, autenticado como el resto de las rutas protegidas.
- Si existe: `200` con la comunidad (`id`, `slug`, `name`, `is_public`).
- Si no existe: `404` con Problem Details.
