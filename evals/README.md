# Katas: corrección y consumo del harness

Tareas repetibles que miden si el agente, con el harness, resuelve bien una
tarea típica y cuánto le cuesta. Los tests de aceptación de cada kata están
ocultos: se copian **después** de que el agente termina, para no confiar en los
tests que escribió él.

Las katas **no** corren en CI: cuestan tokens y requieren autenticación. Se
corren a mano cuando cambian skills, hooks o principios, y el resultado se
registra en el CHANGELOG.

## Estructura

```
evals/<kata>/prompt.md     La tarea, como la escribiría un usuario
evals/<kata>/kata.toml     Plantilla de partida, plugins, setup y comando de aceptación
evals/<kata>/acceptance/   Tests ocultos; imprimen PASS/FAIL por chequeo
evals/results/             Un JSON por configuración: <versión>-<etiqueta>.json
```

| Kata | Plantilla | Aceptación |
|------|-----------|------------|
| `rust-rename-community` | Rust | El dueño renombra (`PATCH`, 200), un no-dueño recibe 403 con Problem Details, y los tests de arquitectura pasan |
| `rust-community-by-slug` | Rust | `GET` por slug encuentra la comunidad; una inexistente da 404 con Problem Details; tests de arquitectura en verde |
| `astro-services-page` | Astro | La página `/servicios/` existe y está en `src/routes.ts`; lint, `astro check` y tests unitarios en verde |

La aceptación de Rust es de caja negra: compila el servidor, lo levanta y le
habla por HTTP. No depende de los nombres internos que elija el agente.

## Correr

```
# Node 24 en el PATH para la kata de Astro
python3 scripts/run_evals.py run --model claude-opus-5-5 --label base
python3 scripts/run_evals.py run --model claude-opus-5-5 --label base --kata astro-services-page
python3 scripts/run_evals.py table evals/results/*.json
```

Cada kata corre 3 veces por configuración (`--runs`), por la variabilidad del
modelo; se informa la mediana y el rango. El modelo se fija siempre con
`--model`. `--parallel N` corre varias ejecuciones a la vez (cada ejecución
tiene su propio `CARGO_TARGET_DIR` en la caché, así que no se bloquean entre sí).

Por ejecución, el runner:

1. Copia los archivos versionados de la plantilla a un directorio nuevo en
   `~/.cache/ai-harness-evals/work/` y hace un commit inicial.
2. Escribe `.claude/settings.json` con `enabledPlugins`: los plugins de la
   kata en `true` y el resto de los plugins del usuario en `false`, para que
   un plugin ajeno no entre en la medición.
3. Corre el `setup` (compilar, `npm ci`), que no cuenta en el consumo.
4. Corre `claude -p` con `--output-format stream-json --verbose`, el modelo
   fijado, `--permission-mode dontAsk` y una lista cerrada de herramientas
   permitidas (`ALLOWED_TOOLS` en el runner), sin `bypassPermissions`.
5. Copia `acceptance/` y la corre.

Registra tokens de entrada, salida, lectura y escritura de caché, costo y
turnos (del evento `result`); skills cargadas, subagentes, herramientas y
archivos leídos (de los `tool_use`, también los de subagentes); bloqueos de
hooks (de `CRAFT_BACKEND_HOOKS_LOG` y `CRAFT_WEB_HOOKS_LOG`), y el resultado
de la aceptación por chequeo. La salida cruda de cada ejecución queda en
`~/.cache/ai-harness-evals/runs/`.

"Archivos leídos" cuenta archivos distintos leídos con `Read`; "del harness",
los que están dentro de este repo (las referencias de las skills, incluida la
plantilla por symlink).

## Resultados

Mediana y, entre paréntesis, el rango de las 3 ejecuciones. Costo en USD según
`total_cost_usd` de Claude Code (precio de lista).

### Línea base: 0.11.0 (`craft-backend` 0.9.0, `craft-web` 0.4.0), Claude Code 2.1.316

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Entrada | Salida | Caché leída | Caché escrita | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| astro-services-page | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.42 ($0.35–$0.42) | 22 (20–23) | 20 (18–24) | 6k (4k–6k) | 291k (249k–341k) | 29k (27k–31k) | 3 (2–4) | 0 | 6 |
| review-planted | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.28 ($0.21–$0.33) | 13 (10–14) | 16 (14–16) | 5k (4k–5k) | 101k (84k–103k) | 20k (14k–27k) | 3 (0–4) | 0 | 0 |
| rust-community-by-slug | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.17 ($1.06–$1.22) | 64 (64–65) | 38 (36–38) | 18k (18k–18k) | 953k (904k–1071k) | 77k (64k–81k) | 20 (20–21) | 0 | 0 |
| rust-rename-community | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.36 ($1.35–$1.37) | 77 (72–80) | 42 (40–50) | 24k (24k–24k) | 1146k (1135k–1449k) | 79k (76k–81k) | 26 (23–29) | 0 | 0 |

- Las 12 corridas pasan la aceptación. Costo total: $9.55.
- Ninguna leyó archivos del harness: en una kata, el proyecto *es* la
  plantilla, y el agente encuentra los ejemplos en su propio directorio.
- Los 6 bloqueos de Astro se repiten igual en las tres corridas. Son de
  `post-edit` al editar `src/routes.ts` antes de que exista la página: la
  ruta sin página deja `routes.test.ts` en rojo un momento.
- Las katas de Rust se repitieron después de corregir el runner. El runner
  copiaba la plantilla conservando los mtimes, y como cada número de corrida
  reutiliza su `CARGO_TARGET_DIR`, cargo no recompilaba `tools/architecture`.
  El test seguía con `env!("CARGO_MANIFEST_DIR")` de otra corrida. Los tests
  de arquitectura, en los hooks y en la aceptación, revisaban otro árbol, o
  entraban en pánico si ese árbol ya no existía. Ahora la copia usa mtimes
  nuevos (test en `test_run_evals.py`).

## Modelos

`review-planted` aplica un diff que compila y pasa clippy y todos los tests,
con cuatro violaciones que solo un revisor ve:

- HEX-7: una regla de negocio en el handler (el prefijo `official-`).
- ERR-2: la variante `Reserved` sin test del handler ni del endpoint.
- RS-2: un `expect` sobre un dato del usuario.
- TDD-1: el slug acepta `_` sin un test que lo exija.

La sesión corre como `architecture-reviewer` (`agent` en `kata.toml`), así que
se mide el modelo del revisor. Una violación cuenta como detectada si una
línea del reporte tiene su ID y, en ese hallazgo, el archivo y una pista que la
identifican.

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Entrada | Salida | Caché leída | Caché escrita | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| astro-services-page | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.42 ($0.35–$0.42) | 22 (20–23) | 20 (18–24) | 6k (4k–6k) | 291k (249k–341k) | 29k (27k–31k) | 3 (2–4) | 0 | 6 |
| review-planted | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.28 ($0.21–$0.33) | 13 (10–14) | 16 (14–16) | 5k (4k–5k) | 101k (84k–103k) | 20k (14k–27k) | 3 (0–4) | 0 | 0 |
| rust-rename-community | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.36 ($1.35–$1.37) | 77 (72–80) | 42 (40–50) | 24k (24k–24k) | 1146k (1135k–1449k) | 79k (76k–81k) | 26 (23–29) | 0 | 0 |
| astro-services-page | 0.11.0-sonnet | claude-sonnet-5-5 | 3/3 | $0.15 ($0.13–$0.18) | 17 (14–19) | 14 (12–24) | 4k (3k–5k) | 160k (127k–331k) | 22k (20k–24k) | 3 (1–4) | 0 | 4 (3–4) |
| review-planted | 0.11.0-sonnet | claude-sonnet-5-5 | 2/3 | $0.10 ($0.09–$0.11) | 5 (5–6) | 8 (6–8) | 3k (2k–3k) | 28k (20k–34k) | 16k (16k–19k) | 1 (1–2) | 0 | 0 |
| rust-rename-community | 0.11.0-sonnet | claude-sonnet-5-5 | 3/3 | $0.58 ($0.57–$0.59) | 61 (56–76) | 32 (30–32) | 22k (21k–22k) | 785k (710k–809k) | 72k (70k–72k) | 11 (5–29) | 0 | 0 |

**Revisor.** Opus detectó las 4 violaciones en las 3 corridas de la línea
base y en 2 de las 3 de `0.11.0-indice`, que no cambia nada que el revisor lea:
5 de 6 en total. Sonnet, en 2 de
3. En la tercera describió ERR-2 y TDD-1 en un hallazgo agrupado
(`[TDD-1 / ERR-2]`) sin ruta, contra el formato del reporte, y etiquetó como
HEX-7 el cambio de `_`. Sonnet cuesta un tercio ($0.10 contra $0.28), usa
menos de la mitad de los turnos y nunca corrió los tests de arquitectura (Opus
los corrió en 1 de 3). Las dos omisiones son de formato, no de contenido. Los dos modelos
describieron la variante `Reserved` sin tests, pero el encabezado del hallazgo
no llevaba ruta, y el criterio, fijado antes de medir, exige ID y archivo en el
mismo hallazgo. No se cumple la condición de fijar el modelo (todas las
violaciones en las 3 corridas): `architecture-reviewer` sigue sin `model:`.
`site-auditor` no tiene una kata equivalente y queda sin cambios.

**Sesión principal.** Con Sonnet, las katas 1 y 3 pasan 3/3, igual que con
Opus. Cuestan menos de la mitad: $0.58 contra $1.36 en Rust y $0.15 contra
$0.42 en Astro, con menos turnos. Los gates deterministas atrapan lo que la
aceptación mide.

**Recomendación.** Para implementar un encargo bien especificado, Sonnet
rinde igual en estas katas por menos de la mitad del costo. Para revisar,
Opus: la revisión es barata en cualquier modelo, y lo que cuenta es no dejar
pasar una violación. El harness no fija el modelo de la sesión principal; se
elige con `/model` o `--model`.

## Índice de ejemplos (`references/exemplars.md`)

Con Opus, contra la línea base:

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Entrada | Salida | Caché leída | Caché escrita | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| astro-services-page | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.42 ($0.35–$0.42) | 22 (20–23) | 20 (18–24) | 6k (4k–6k) | 291k (249k–341k) | 29k (27k–31k) | 3 (2–4) | 0 | 6 |
| rust-rename-community | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.36 ($1.35–$1.37) | 77 (72–80) | 42 (40–50) | 24k (24k–24k) | 1146k (1135k–1449k) | 79k (76k–81k) | 26 (23–29) | 0 | 0 |
| astro-services-page | 0.11.0-indice | claude-opus-5-5 | 3/3 | $0.42 ($0.36–$0.48) | 23 (19–25) | 24 (20–28) | 6k (5k–6k) | 347k (266k–443k) | 30k (26k–33k) | 4 (1–4) | 1 | 6 (3–6) |
| rust-rename-community | 0.11.0-indice | claude-opus-5-5 | 3/3 | $1.50 ($1.49–$1.51) | 81 (80–87) | 62 (50–64) | 26k (26k–28k) | 1851k (1458k–1893k) | 77k (77k–80k) | 23 (23–25) | 0 | 0 (0–1) |

- La aceptación no cambia: 3/3 en las dos katas, y también en
  `rust-community-by-slug` (ver `0.11.0-indice.json`).
- El agente leyó el índice en todas las corridas de Astro y de la consulta por
  slug, y en ninguna del renombre.
- No hay ahorro medible. Era lo esperado: en una kata, el proyecto es la
  plantilla, y la línea base ya leía 0 archivos del harness. El renombre
  costó un 10 % más ($1.50 contra $1.36), con rangos estrechos en las dos
  configuraciones. Pero ninguna de esas corridas leyó el índice, así que la
  diferencia no viene de él.
- Medir el ahorro real pide una kata sobre un proyecto que ya se apartó de la
  plantilla.

## Configuración final contra la línea base (0.12.0)

0.12.0 publica el índice de ejemplos (C) y no fija modelos (B). `CLAUDE.md`
(D) no llega a los plugins. Así, la configuración final es la de
`0.11.0-indice`: los mismos plugins, sin el cambio de versión, que no altera
el comportamiento. Opus, 3 corridas por kata:

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Entrada | Salida | Caché leída | Caché escrita | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| astro-services-page | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.42 ($0.35–$0.42) | 22 (20–23) | 20 (18–24) | 6k (4k–6k) | 291k (249k–341k) | 29k (27k–31k) | 3 (2–4) | 0 | 6 |
| astro-services-page | 0.11.0-indice | claude-opus-5-5 | 3/3 | $0.42 ($0.36–$0.48) | 23 (19–25) | 24 (20–28) | 6k (5k–6k) | 347k (266k–443k) | 30k (26k–33k) | 4 (1–4) | 1 | 6 (3–6) |
| review-planted | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.28 ($0.21–$0.33) | 13 (10–14) | 16 (14–16) | 5k (4k–5k) | 101k (84k–103k) | 20k (14k–27k) | 3 (0–4) | 0 | 0 |
| review-planted | 0.11.0-indice | claude-opus-5-5 | 2/3 | $0.27 ($0.25–$0.36) | 12 (12–16) | 16 (14–16) | 5k (5k–5k) | 103k (82k–105k) | 18k (17k–30k) | 3 (3–6) | 0 | 0 |
| rust-community-by-slug | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.17 ($1.06–$1.22) | 64 (64–65) | 38 (36–38) | 18k (18k–18k) | 953k (904k–1071k) | 77k (64k–81k) | 20 (20–21) | 0 | 0 |
| rust-community-by-slug | 0.11.0-indice | claude-opus-5-5 | 3/3 | $1.16 ($1.03–$1.21) | 64 (63–69) | 40 (38–44) | 19k (18k–19k) | 987k (838k–1219k) | 73k (63k–75k) | 22 (21–23) | 1 | 0 |
| rust-rename-community | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.36 ($1.35–$1.37) | 77 (72–80) | 42 (40–50) | 24k (24k–24k) | 1146k (1135k–1449k) | 79k (76k–81k) | 26 (23–29) | 0 | 0 |
| rust-rename-community | 0.11.0-indice | claude-opus-5-5 | 3/3 | $1.50 ($1.49–$1.51) | 81 (80–87) | 62 (50–64) | 26k (26k–28k) | 1851k (1458k–1893k) | 77k (77k–80k) | 23 (23–25) | 0 | 0 (0–1) |

- Aceptación: 12/12 en la línea base y 11/12 en la final. La diferencia es la
  revisión de formato descrita en "Modelos".
- Costo y turnos: iguales dentro del rango, salvo `rust-rename-community`
  (+10 %, sin relación con el índice).
- Archivos leídos: la final lee además el índice (1 archivo del harness) en
  Astro y en la consulta por slug.

Las palancas de bajo riesgo no mueven el consumo de estas katas. El dato que
sí lo mueve es el modelo de la sesión principal: con Sonnet, las katas 1 y 3
cuestan menos de la mitad con la misma aceptación (ver "Modelos").
