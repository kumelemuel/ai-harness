# Stack — Rust

## Detalles

Fuera de una sección de verificación no se comprueba: `hex_9_fuera_de_seccion`.

## Enforcement

| Regla | Mecanismo |
|-------|-----------|
| Existe con archivo y comodín | `tests/naming.rs::hex_4_*` |
| Existe por nombre exacto | `hex_4_use_cases_are_single_method` |
| Comodín sin archivo | `hex_2_core_*` |
| Test de unidad con `#[test]` fuera de `tests/` | `rs_3_unit_*` |
| Archivo de test | `tests/naming.rs` |
| Función que no existe | `hex_9_missing` |
| Archivo que existe sin esa función | `tests/naming.rs::hex_6_*` |
| Archivo que no existe | `tests/gone.rs::hex_4_*` |
| Comodín que solo coincide con un helper que no es test | `err_1_*` |
| Archivo de test que no existe | `tests/errors.rs` |

```text
dentro de código no cuenta: `hex_9_en_codigo`
```

## Otra sección

`hex_9_despues_de_la_seccion` no se comprueba.
