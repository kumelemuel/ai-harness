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
| rust-community-by-slug | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.20 ($1.09–$1.35) | 65 (59–71) | 34 (32–48) | 18k (18k–20k) | 985k (710k–1463k) | 79k (74k–83k) | 22 (20–24) | 0 | 0 (0–10) |
| rust-rename-community | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.60 ($1.44–$1.67) | 85 (78–91) | 60 (46–74) | 28k (26k–29k) | 1715k (1315k–2320k) | 82k (81k–85k) | 25 (25–26) | 0 | 0 (0–3) |

- Las 9 corridas pasan la aceptación.
- El costo total de las 9 corridas fue de $9.54.
- Ninguna corrida leyó archivos del harness: en una kata, el proyecto *es* la
  plantilla, así que el agente lee su propio directorio de trabajo.
- Los 6 bloqueos de Astro se repiten igual en las tres corridas: `post-edit`
  al editar `src/routes.ts` antes de que exista la página.
  `routes.test.ts` queda en rojo un momento.

Una ejecución cortada por la API (`api_error_status`, por ejemplo un 429 por
límite de uso) se marca inválida, queda fuera de la mediana y se repite al
volver a correr con la misma etiqueta (`--fresh` repite también las válidas).
