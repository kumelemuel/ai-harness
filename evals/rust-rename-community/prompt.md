Agrega el caso de uso para renombrar una comunidad. Solo el dueño puede hacerlo.

Contrato HTTP:

- `PATCH /communities/{id}` con el cuerpo `{"name": "Nuevo nombre"}`, autenticado
  como el resto de las rutas protegidas.
- Si el que llama es el dueño: `200` con la comunidad actualizada
  (`id`, `slug`, `name`, `is_public`).
- Si no es el dueño: `403` con Problem Details.
