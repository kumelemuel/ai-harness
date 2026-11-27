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

**Control de otra versión del harness.** `--harness-ref REF` carga los plugins
de la kata desde `REF` (extraído con `git archive` a la caché), con los del
marketplace deshabilitados. La plantilla sigue saliendo del clon, así que la
única variable es el plugin. Las dos configuraciones corren en paralelo, en la
misma ventana:

```sh
python3 scripts/run_evals.py run --model claude-sonnet-5-5 --label rama --kata rust-rename-community &
python3 scripts/run_evals.py run --model claude-sonnet-5-5 --label control --harness-ref main --kata rust-rename-community &
wait
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
`total_cost_usd` de Claude Code (precio de lista). Cada tabla muestra solo su
configuración; la referencia es siempre la [línea base](#línea-base).

### Línea base

0.11.0 (`craft-backend` 0.9.0, `craft-web` 0.4.0), Claude Code 2.1.316, Opus 5.5.

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| astro-services-page | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.42 ($0.35–$0.42) | 22 (20–23) | 10 (9–12) | 20 (18–24) | 6k (4k–6k) | 291k (249k–341k) | 29k (27k–31k) | 22k (20k–24k) | 3 (2–4) | 0 | 6 |
| review-planted | 0.11.0-base | claude-opus-5-5 | 3/3 | $0.28 ($0.21–$0.33) | 13 (10–14) | 8 (7–8) | 16 (14–16) | 5k (4k–5k) | 101k (84k–103k) | 20k (14k–27k) | 37k (26k–41k) | 3 (0–4) | 0 | 0 |
| rust-community-by-slug | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.17 ($1.06–$1.22) | 64 (64–65) | 19 (18–19) | 38 (36–38) | 18k (18k–18k) | 953k (904k–1071k) | 77k (64k–81k) | 97k (65k–110k) | 20 (20–21) | 0 | 0 |
| rust-rename-community | 0.11.0-base | claude-opus-5-5 | 3/3 | $1.36 ($1.35–$1.37) | 77 (72–80) | 21 (20–25) | 42 (40–50) | 24k (24k–24k) | 1146k (1135k–1449k) | 79k (76k–81k) | 86k (81k–91k) | 26 (23–29) | 0 | 0 |

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

### Modelos

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

Sonnet 5.5, mismo harness que la línea base:

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| astro-services-page | 0.11.0-sonnet | claude-sonnet-5-5 | 3/3 | $0.15 ($0.13–$0.18) | 17 (14–19) | 7 (6–12) | 14 (12–24) | 4k (3k–5k) | 160k (127k–331k) | 22k (20k–24k) | 11k (11k–17k) | 3 (1–4) | 0 | 4 (3–4) |
| review-planted | 0.11.0-sonnet | claude-sonnet-5-5 | 2/3 | $0.10 ($0.09–$0.11) | 5 (5–6) | 4 (3–4) | 8 (6–8) | 3k (2k–3k) | 28k (20k–34k) | 16k (16k–19k) | 20k (18k–26k) | 1 (1–2) | 0 | 0 |
| rust-rename-community | 0.11.0-sonnet | claude-sonnet-5-5 | 3/3 | $0.58 ($0.57–$0.59) | 61 (56–76) | 16 (15–16) | 32 (30–32) | 22k (21k–22k) | 785k (710k–809k) | 72k (70k–72k) | 76k (75k–77k) | 11 (5–29) | 0 | 0 |

**Revisor.** Opus detectó las 4 violaciones en 5 de 6 corridas: las 3 de la
línea base y 2 de las 3 de [`0.11.0-indice`](#índice-de-ejemplos-archivado),
cuyo revisor no lee nada de lo que cambia esa configuración. Sonnet las
detectó en 2 de 3.

- Las dos omisiones son de formato. Los dos modelos describieron la variante
  `Reserved` sin tests, pero el encabezado del hallazgo no llevaba ruta, y el
  criterio, fijado antes de medir, exige ID y archivo en el mismo hallazgo.
- Además, Sonnet etiquetó como HEX-7 el cambio de `_`, que corresponde a
  TDD-1. Ese es un error de contenido.
- Sonnet cuesta un tercio ($0.10 contra $0.28) y nunca corrió los tests de
  arquitectura. Opus los corrió en 1 de 3.
- No se cumple la condición para fijar el modelo (todas las violaciones en las
  3 corridas): `architecture-reviewer` sigue sin `model:`. `site-auditor` no
  tiene una kata equivalente.

**Sesión principal.** Con Sonnet, las katas 1 y 3 pasan 3/3, igual que con
Opus, por menos de la mitad del costo ($0.58 contra $1.36 en Rust y $0.15
contra $0.42 en Astro). La aceptación no ve lo que solo ve un revisor (HEX-7,
TDD-1, ERR-2). Por eso la recomendación espera la revisión cruzada.

#### Formato estricto del revisor

`architecture-reviewer` y `site-auditor` exigen una línea por hallazgo:
`ID | ruta:línea | descripción`, un ID por línea y el detalle indentado
debajo. La aceptación conserva el criterio (ID, archivo y pista en el mismo
hallazgo) y solo adapta el parseo. La única variable respecto de
`0.11.0-base` y `0.11.0-sonnet` es el prompt del revisor.

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| review-planted | 0.12.0-formato | claude-opus-5-5 | 3/3 | $0.34 ($0.32–$0.41) | 15 (12–16) | 8 (7–11) | 16 (14–22) | 5k (5k–6k) | 96k (85k–193k) | 27k (26k–32k) | 41k (39k–48k) | 4 (4–5) | 0 | 0 |

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| review-planted | 0.12.0-formato-sonnet | claude-sonnet-5-5 | 3/3 | $0.08 ($0.08–$0.09) | 4 (4–5) | 4 | 8 | 2k (2k–3k) | 28k (28k–29k) | 13k (13k–16k) | 13k (10k–18k) | 0 (0–1) | 0 | 0 |

- **Los dos modelos detectaron las 4 violaciones en las 3 corridas.** El
  formato eliminó las omisiones por hallazgos agrupados, y Sonnet ya no
  etiqueta como HEX-7 el cambio de `_`.
- **Sonnet agregó 3 hallazgos con el ID equivocado,** en 2 de 3 corridas.
  Son observaciones razonables sobre el 409 de `Reserved`, pero las etiquetó
  como HEX-1, HEX-3 y HEX-8, que tratan de dependencias, puertos y kernel.
  Opus no tuvo hallazgos fuera de las reglas plantadas.
- **Costo:** Sonnet $0.08 contra $0.34 de Opus.
- **Decisión (revertida):** se cumplía la regla y `architecture-reviewer` fijó
  `model: claude-sonnet-5-5`. Se revirtió porque esta medición corrió sin las
  skills del revisor; se repite en modo subagente. Verificado con una sesión real: la sesión
  principal en Opus invoca al subagente, y `modelUsage` muestra al subagente
  en Sonnet.
- **Salvedad:** este revisor corrió con `--agent`, sin sus skills (ver
  "Limitación: `--agent` no precarga skills"). En uso real, como subagente,
  sí las tiene. La decisión debe validarse en ese modo. `site-auditor` cambia de formato pero no de modelo, porque no
  tiene una kata equivalente.

#### Revisión cruzada: Sonnet implementa, Opus revisa

`run_evals.py review` reconstruye el diff final de cada corrida (la plantilla
más el `diff.patch` que guarda el runner, contra su commit inicial) y lo revisa
con `architecture-reviewer` en Opus, con el formato estricto. Se corrió sobre
`rust-rename-community`: 3 corridas de Opus como control y 3 de Sonnet.

Implementaciones:

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rust-rename-community | 0.12.0-diffs-opus | claude-opus-5-5 | 3/3 | $1.49 ($1.33–$1.63) | 77 (74–78) | 21 (20–27) | 42 (40–54) | 25k (24k–26k) | 1335k (1126k–1859k) | 91k (78k–93k) | 115k (85k–117k) | 28 (25–30) | 0 | 0 |

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rust-rename-community | 0.12.0-diffs-sonnet | claude-sonnet-5-5 | 3/3 | $0.64 ($0.52–$0.67) | 64 (64–75) | 19 (12–22) | 38 (24–44) | 23k (19k–25k) | 1118k (544k–1193k) | 73k (69k–77k) | 76k (73k–79k) | 24 (13–26) | 0 | 0 |

Revisiones (Opus) de esos diffs:

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rust-rename-community-revision | 0.12.0-revision-de-opus | claude-opus-5-5 | 3/3 | $0.47 ($0.41–$0.52) | 24 (23–26) | 14 (13–15) | 28 (26–30) | 7k (6k–8k) | 320k (290k–340k) | 33k (29k–36k) | 58k (52k–63k) | 10 (9–12) | 0 | 0 |

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rust-rename-community-revision | 0.12.0-revision-de-sonnet | claude-opus-5-5 | 3/3 | $0.45 ($0.43–$0.47) | 24 (22–24) | 13 (12–14) | 26 (24–28) | 7k (6k–7k) | 273k (234k–300k) | 30k (30k–37k) | 55k (51k–56k) | 11 | 0 | 0 |

Hallazgos, clasificados a mano (pendientes de revisión):

| Diff de | Corrida | Hallazgo | Clasificación | Justificación |
|---|---|---|---|---|
| Opus | 1 | HEX-7 `api/src/rename_community.rs:85` | falso positivo | El endpoint traduce una hoja `Forbidden` a su mensaje y código; es traducción de transporte, no una decisión de negocio |
| Opus | 1 | TDD-1 `apps/server/src/main.rs:47` | falso positivo | Cablear sin test es la convención de la plantilla (`server` no tiene tests), y el revisor lo admite |
| Opus | 2, 3 | ninguno | — | — |
| Sonnet | 1 | TDD-1 `rename_community/handler.rs:46` | válido | Ningún test cubre que un fallo de `save` se propague como `Unexpected`: el doble falla ya en `find_by_id` |
| Sonnet | 2 | ninguno | — | — |
| Sonnet | 3 | TDD-1 `apps/server/src/main.rs:47` | falso positivo | El mismo caso que en el diff de Opus, corrida 1 |

**Hallazgos válidos por corrida:** Opus 0, 0, 0; Sonnet 1, 0, 0.

**Costo combinado** (medianas):

| Flujo | Costo |
|---|---|
| Opus implementa | $1.49 |
| Sonnet implementa ($0.64) + Opus revisa ($0.45) | $1.09 |

**Recomendación:** en esta kata los hallazgos válidos son comparables, así que
conviene "Sonnet implementa, Opus revisa", que cuesta un 27 % menos que Opus
solo.

**Salvedad:** el revisor corrió sin sus skills (ver "Limitación: `--agent`
no precarga skills"). Hay que repetirlo en modo subagente antes de adoptar el
flujo.

#### Limitación: `--agent` no precarga skills

Una sesión con `--agent craft-backend:architecture-reviewer` **no carga** las
`skills:` de su frontmatter. Invocado como subagente desde una sesión
principal, sí las carga. Verificado con Haiku y sin herramientas, pidiendo
copiar una línea de `hexagonal-architecture`:

- con `--agent`, la sesión responde `NO-CARGADA`;
- como subagente, la copia literalmente.

Todas las mediciones del revisor (`review-planted` y estas revisiones)
midieron un revisor sin skills, que nunca llamó a `Skill` y que en 7
reportes dice no encontrarlas. `CLAUDE_CODE_SUBAGENT_MODEL` no se impone a
`model:`: medir el subagente con otro modelo requiere cambiar su `model:`
por configuración.

#### Revisor como subagente (con sus skills)

`review-planted-subagent`: una sesión mínima en Haiku solo invoca a
`architecture-reviewer`. El plugin se carga desde una copia con el `model:`
del revisor fijado (`--subagent-model`, `--plugin-dir`). El reporte que juzga
la aceptación es el `tool_result` del subagente, no lo que dice la sesión.
Opus y Sonnet corrieron intercalados, de a dos, en la misma ventana.

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| review-planted-subagent@claude-opus-5-5 | 0.12.0-subagente | claude-haiku-5-5 | 3/3 | $0.23 ($0.21–$0.25) | 2 | 9 (7–9) | 4 | 0k (0k–1k) | 28k (28k–28k) | 14k (13k–14k) | 35k (33k–36k) | 0 | 0 | 0 |
| review-planted-subagent@claude-sonnet-5-5 | 0.12.0-subagente | claude-haiku-5-5 | 1/3 | $0.06 ($0.06–$0.07) | 2 | 4 (4–6) | 4 | 0k (0k–0k) | 28k (28k–28k) | 13k (13k–13k) | 15k (13k–15k) | 0 | 0 | 0 |

**Detección:** Opus encontró las 4 violaciones en 3/3 corridas; Sonnet, en
1/3. En las otras dos, Sonnet volvió a etiquetar como HEX-7 el `_` sin test,
que es TDD-1.

**Precisión:** hallazgos ajenos a las violaciones plantadas, clasificados a
mano.

| Modelo | Corrida | Hallazgo | Clasificación |
|---|---|---|---|
| Opus | 1–3 | Extras: TDD-1 de la regla de reserva y de `reserved()` sin tests, una segunda línea HEX-7 para `staff-`, ERR-2 del endpoint | válidos |
| Sonnet | 1 | ERR-2 `errors.rs:16`: "la hoja duplica `Conflict`" | falso positivo (opinión, sin regla violada) |
| Sonnet | 1 | HEX-7 `community_slug.rs:17` | ID equivocado (es TDD-1) |
| Sonnet | 1 | ERR-4 `create_community.rs:98`: "observación sin infracción" | falso positivo (lo admite el propio reporte) |
| Sonnet | 2 | HEX-7 `community_slug.rs:19` | ID equivocado (es TDD-1) |
| Sonnet | 2 | ERR-1 `handler.rs:47`: semántica parecida a `Conflict` | falso positivo |
| Sonnet | 2 | VER-1 sin ruta: "no ejecuté tests" | falso positivo |
| Sonnet | 3 | HEX-7 `create_community.rs:98`: "es una traducción de borde" | falso positivo (lo admite el propio reporte) |

En contra: Opus 0, Sonnet 7.

**Decisión:** Sonnet no cumple ni la detección ni la precisión; Opus cumple
las dos. `architecture-reviewer` fija `model: claude-opus-5-5` (la regla vale en
cualquier dirección, ver `CLAUDE.md`). Sin la fijación, el flujo recomendado
("Sonnet implementa, Opus revisa") correría el revisor en Sonnet, porque el
subagente hereda el modelo de la sesión. Costo del revisor: Opus $0.22, Sonnet
$0.05.

**La fijación surte efecto** (`0.13.0-fijacion-opus`): la kata con la sesión
principal en Sonnet y el plugin desde el clon, sin `--subagent-model`. Las 7
peticiones del subagente corrieron en `claude-opus-5-5` (`modelUsage`: Opus
$0.24, Sonnet $0.05) y la aceptación pasó.

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| review-planted-subagent | 0.13.0-fijacion-opus | claude-sonnet-5-5 | 1/1 | $0.29 | 2 | 7 | 4 | 0k | 27k | 12k | 33k | 0 | 0 | 0 |

#### Revisión cruzada con el revisor como subagente

`run_evals.py review` en modo subagente (Opus, con sus skills), sobre los
mismos diffs de `0.12.0-diffs-opus` y `0.12.0-diffs-sonnet`:

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rust-rename-community-revision | 0.12.0-revision-sub-de-opus | claude-haiku-5-5 | 3/3 | $0.55 ($0.44–$0.58) | 2 (2–8) | 13 (10–21) | 4 (4–16) | 0k (0k–3k) | 28k (18k–161k) | 23k (13k–34k) | 90k (73k–92k) | 20 (13–23) | 0 | 0 (0–1) |

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rust-rename-community-revision | 0.12.0-revision-sub-de-sonnet | claude-haiku-5-5 | 3/3 | $0.49 ($0.42–$0.56) | 2 (2–9) | 13 (11–23) | 4 (4–18) | 0k (0k–3k) | 18k (18k–207k) | 23k (23k–25k) | 80k (57k–92k) | 14 (12–14) | 0 | 0 (0–1) |

| Diff de | Corrida | Hallazgo | Clasificación | Justificación |
|---|---|---|---|---|
| Opus | 1 | VER-1 `tools/architecture/tests/naming.rs:1` | falso positivo (del entorno) | Es real, pero sobre el runner, no sobre el diff: los tests que corrió no eran los de su árbol (ver abajo) |
| Opus | 1 | VER-1 `api/tests/rename_community.rs:24` | válido | `StubRenameUseCase` guarda los comandos que recibe: es un `Spy` con nombre de `Stub`, y ninguna regla lo ve |
| Opus | 2, 3 | ninguno | — | — |
| Sonnet | 1 | ninguno | — | — |
| Sonnet | 2 | TDD-1 `apps/server/src/main.rs:47` | falso positivo | Cablear sin test es la convención de la plantilla |
| Sonnet | 3 | ERR-4 `api/src/rename_community.rs:41` | falso positivo | El rechazo de los extractores de axum es el mismo patrón que `create_community` en la plantilla, no algo del diff |
| Sonnet | 3 | TDD-1 `apps/server/src/main.rs:42` | falso positivo | Convención de la plantilla |

**Hallazgos válidos por corrida:** diffs de Opus 1, 0, 0; diffs de Sonnet 0,
0, 0.

**Costo combinado:** Sonnet implementa ($0.64) + Opus revisa como subagente
($0.49) = $1.13, contra $1.49 de "Opus implementa". Es un 24 % menos, con
hallazgos válidos comparables. Se recomienda ese flujo.

**Salvedad del entorno:** las dos revisiones se corrieron en paralelo y, con
la misma kata, el mismo modelo y el mismo número de corrida, compartieron
`CARGO_TARGET_DIR`. El revisor de la corrida 1 lo detectó: los tests que
corrió eran de otro árbol. La lectura del diff es válida; las corridas de
tests dentro de esas revisiones, no. `cache_slot` incluye ahora la etiqueta.

**Consecuencias en 0.13.0:** los cuatro falsos positivos TDD-1 sobre
`apps/server/src/main.rs` (dos en cada modo de revisión) llevaron a dejar escrito que el composition root no
lleva test unitario propio (TDD-1 y el revisor). El `Spy` con nombre de `Stub`
llevó a asignar la semántica de los prefijos de dobles al revisor
(`core/verification.md`): el test de nombres ve el prefijo, no lo que hace el
doble.

### Línea base de 0.14 (plantilla con `iam`, eventos y suscriptores)

0.14.0 (implementación) y 0.14.1 (revisión; solo cambia `planted.diff`),
`craft-backend` 0.11.0, Claude Code 2.1.316. Flujo adoptado: Sonnet
implementa y Opus revisa como subagente, con sus skills.

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rust-community-by-slug | 0.14.0-sonnet | claude-sonnet-5-5 | 3/3 | $0.42 ($0.40–$0.44) | 49 (49–58) | 11 (10–11) | 22 (20–22) | 15k (15k–18k) | 410k (374k–416k) | 56k (52k–57k) | 58k (56k–60k) | 20 (19–24) | 0 | 0 |
| rust-rename-community | 0.14.0-sonnet | claude-sonnet-5-5 | 3/3 | $0.68 ($0.64–$0.73) | 60 (49–75) | 18 (17–22) | 36 (34–44) | 26k (23k–28k) | 1055k (929k–1336k) | 80k (79k–80k) | 84k (78k–89k) | 13 (1–24) | 0 | 0 |
| review-planted-subagent@claude-opus-5-5 | 0.14.1-subagente | claude-haiku-5-5 | 3/3 | $0.25 ($0.24–$0.26) | 2 | 9 (8–10) | 4 | 1k (0k–1k) | 28k | 14k (13k–14k) | 38k (30k–41k) | 0 (0–1) | 0 | 0 |
| rust-community-by-slug-revision | 0.14.1-revision-de-sonnet | claude-haiku-5-5 | 3/3 | $0.47 ($0.45–$0.50) | 2 | 15 (12–16) | 4 | 0k (0k–1k) | 28k (28k–28k) | 13k (13k–14k) | 63k (52k–94k) | 12 (9–13) | 0 | 0 |
| rust-rename-community-revision | 0.14.1-revision-de-sonnet | claude-haiku-5-5 | 3/3 | $0.56 ($0.55–$0.62) | 2 | 12 (10–14) | 4 | 1k (0k–1k) | 28k (28k–28k) | 14k (13k–15k) | 106k (96k–112k) | 16 (14–21) | 0 | 0 |

- **No es comparable con la línea base de 0.11.0:** cambiaron a la vez las
  reglas y la plantilla (dos variables), y el runner carga los plugins desde el
  clon, así que no pudo correr 0.13.0 como control en la misma ventana. Es una
  línea base nueva: pasan la implementación (3/3 y 3/3), el revisor sobre el
  diff plantado (las 4 violaciones en las 3 corridas) y la revisión cruzada.
- **Costo combinado (medianas):** renombrar $1.24 (Sonnet $0.68 + Opus
  $0.56); buscar por slug $0.89 ($0.42 + $0.47).
- **Corridas inválidas:** el límite de 5 horas cortó con 429 las primeras
  corridas de `0.14.1-subagente` (el revisor ya había reportado, pero la sesión
  principal recibió el 429) y todas las de `0.14.1-revision-de-sonnet`, a $0 y
  sin subagente. Se repitieron con la misma etiqueta.

Hallazgos de la revisión cruzada, clasificados a mano y aprobados por el
usuario. Por corrida: renombrar 3, 5 y 3; buscar por slug 0, 4 y 0.

| Hallazgo | Veces | Clasificación | Justificación |
|---|---|---|---|
| TDD-1: ningún test exige "publicar solo si guardó" en `RenameCommunityHandler` | 1 | válido | La plantilla tiene ese test en `CreateCommunityHandler`; el agente no lo replicó |
| TDD-2: `StubUseCase` registra lo que recibe y el test lo afirma | 3 | válido | Es un `Spy` con nombre de `Stub`. La plantilla tenía el mismo caso en el middleware de `iam` |
| TDD-2: dobles `Unused…UseCase` sin prefijo de la lista cerrada | 6 | válido (hueco del harness) | TDD-2 decía "todos los dobles", pero no daba forma a los de puertos entrantes, y la plantilla usaba `StubUseCase` |
| VER-1: `hex_5_adapters_*` no revisa los dobles de puertos entrantes | 4 | válido sobre el harness, no sobre el diff | El mismo hueco, visto desde el test |
| TDD-2: `FakeModerationAdapter` usado como `Spy` (`reviewed()`) | 1 | falso positivo | Un `Fake` puede exponer lo que recibió: es parte de su estado |
| VER-1: no hay un test que exija un test por variante del error | 1 | falso positivo | Es una decisión documentada: lo verifica el revisor (ERR-2) |

**Consecuencia en 0.15.0:** TDD-2 da forma a los dobles de puertos entrantes
(`{Fake|Fixed|Stub|Spy}{Puerto}`, sin `Adapter`) con el test
`tdd_2_inbound_doubles_are_named_by_prefix_and_port`, y la plantilla renombra
sus dos `StubUseCase`. Aclara además que un `Fake` puede exponer lo que
recibió.

### Candidata 1.0.0-rc.4 (contra 1.0.0-rc.3)

`main` en `ca63d84` (1.0.0-rc.3.8) contra `v1.0.0-rc.3` (`52abe35`) con
`--harness-ref`, 2026-11-26, las dos configuraciones en paralelo kata por kata.
Sonnet implementa; en la revisión, Haiku en la sesión y el revisor en Opus (su
`model:`). Entre las dos cambian las skills `tdd`, `error-handling` y
`hexagonal-architecture`, el revisor (TDD-1 y RS-2), los hooks `post-edit` y
`Stop` (mensajes) y los principios TDD-1, ERR-2, ERR-4, HEX-1, HEX-2, HEX-4 y
HEX-8. La plantilla sale del clon en las dos.

| Kata | Configuración | Aceptación | Costo | Turnos | Peticiones |
|---|---|---|---|---|---|
| rust-rename-community | main | 3/3 | $0.76 ($0.64–$0.87) | 76 (66–81) | 22 (15–26) |
| rust-rename-community | control | 3/3 | $0.76 ($0.67–$0.90) | 56 (49–78) | 21 (15–26) |
| rust-community-by-slug | main | 3/3 | $0.49 ($0.42–$0.52) | 55 (49–59) | 13 (11–14) |
| rust-community-by-slug | control | 3/3 | $0.53 ($0.50–$0.56) | 60 (53–64) | 15 (13–16) |
| review-planted-subagent | main | 3/3 | $0.25 ($0.25–$0.28) | 3 (3–8) | 10 (9–13) |
| review-planted-subagent | control | 3/3 | $0.23 ($0.16–$0.27) | 4 (3–7) | 9 (6–11) |

- Los rangos se solapan en las tres katas: ningún efecto establecido. En slug,
  apenas ($0.50–$0.52).
- El revisor detectó las cuatro violaciones plantadas (HEX-7, ERR-2, RS-2,
  TDD-1) en las 6 corridas. Un bloqueo del gate de Stop por corrida de revisión
  en las dos configuraciones, como en rc.3 (`rs_2_expect_names_the_invariant`).
- Una corrida de renombre de `main` tuvo 3 avisos de `post-edit` y terminó en
  verde; ninguna otra tuvo bloqueos de `post-edit`.
- Corridas: una primera ronda solo de `main` (`1.0.0-rc4-*.json`, sin control
  por un error de argumentos del lanzador: 3/3 en las tres, $0.74, $0.56 y
  $0.28) no se compara. La ronda intercalada (`1.0.0-rc4b-*.json`) se cortó por
  el límite de uso: 16 corridas inválidas, repetidas con la misma etiqueta en
  otra ventana, las dos configuraciones juntas. Costo total, con la ronda sin
  control y las inválidas: unos $14.40.

### Candidata 1.0.0-rc.3 (contra 1.0.0-rc.2)

`main` en `32e2ca3` (1.0.0-rc.2.8) contra `v1.0.0-rc.2` (`451fec9`) con
`--harness-ref`, 2026-11-25, las dos configuraciones en paralelo kata por kata.
Sonnet implementa; en la revisión, Haiku en la sesión y el revisor en Opus (su
`model:`). Entre las dos cambian las skills `tdd`, `rust-stack`,
`hexagonal-architecture` y `adoption`, el revisor (prioridades 5 y 6), el hook
`pre-edit` y los principios TDD, ERR-1, HEX-1, HEX-4, HEX-6 y HEX-8. La
plantilla sale del clon en las dos.

| Kata | Configuración | Aceptación | Costo | Turnos | Archivos leídos |
|---|---|---|---|---|---|
| rust-rename-community | main | 3/3 | $0.73 ($0.67–$0.87) | 61 (56–67) | 7 (1–24) |
| rust-rename-community | control | 3/3 | $0.77 ($0.73–$1.10) | 50 (1–73) | 15 (0–18) |
| rust-community-by-slug | main | 3/3 | $0.50 ($0.49–$0.51) | 56 (52–59) | 20 (10–22) |
| rust-community-by-slug | control | 3/3 | $0.54 ($0.54–$0.60) | 58 (57–66) | 20 (19–24) |
| review-planted-subagent | main | 3/3 | $0.24 ($0.23–$0.29) | 4 (3–4) | 0 |
| review-planted-subagent | control | 3/3 | $0.27 ($0.25–$0.31) | 3 (3–5) | 1 (1–2) |

- Renombre y revisión: los rangos se solapan, ningún efecto establecido.
- **Slug: los rangos no se solapan** ($0.49–$0.51 frente a $0.54–$0.60): con
  la regla de 3 corridas, `main` es más barato en esta kata. Las diferencias
  de texto que podrían explicarlo son varias (cambian varias skills a la vez),
  así que el efecto no se atribuye a ninguna en particular. La corrida 3 de
  cada configuración se repitió en otra ventana (429), las dos juntas.
- El revisor detectó las cuatro violaciones plantadas (HEX-7, ERR-2, RS-2,
  TDD-1) en las 6 corridas.
- Un bloqueo del gate de Stop por corrida de revisión, en las dos
  configuraciones: `rs_2_expect_names_the_invariant`, la regla nueva, detecta
  el `expect` plantado (RS-2). Depende de la plantilla, no de los plugins.
- La primera ventana se cortó por el límite de uso (429): 18 corridas
  inválidas, repetidas con la misma etiqueta. Costo total, con las inválidas:
  $9.63. Los archivos de resultados se renombraron a `1.0.0-rc3-*.json`; la
  etiqueta dentro del JSON es `1.0.0-1.0.0-rc3-*`.

### Candidata 1.0.0-rc.2 (contra 1.0.0-rc.1)

`main` en `9810b44` (1.0.0-rc.2) contra `v1.0.0-rc.1` (`e9526c8`) con
`--harness-ref`, 2026-11-23 de 06:32 a 06:54, las dos configuraciones en
paralelo kata por kata. Sonnet implementa; en la revisión, Haiku en la sesión y
el revisor en Opus (su `model:`). Entre las dos cambian la skill `adoption`
(modo `sincronizar`, "Ritual de rebanada"), un hook `SessionStart` nuevo que
sin `CRAFT_BACKEND_BASELINE_INIT=1` no imprime nada, el mensaje de bloqueo de
`pre-edit` y `legacy.rs`.

| Kata | Configuración | Aceptación | Costo | Turnos | Archivos leídos |
|---|---|---|---|---|---|
| rust-rename-community | main | 3/3 | $0.72 ($0.60–$0.76) | 58 (50–79) | 2 (1–27) |
| rust-rename-community | control | 3/3 | $0.72 ($0.71–$0.78) | 63 (59–75) | 11 (7–26) |
| rust-community-by-slug | main | 3/3 | $0.49 ($0.44–$0.53) | 59 (48–61) | 16 (13–23) |
| rust-community-by-slug | control | 3/3 | $0.45 ($0.39–$0.58) | 53 (48–56) | 20 (15–24) |
| review-planted-subagent | main | 3/3 | $0.27 ($0.19–$0.31) | 2 | 0 (0–2) |
| review-planted-subagent | control | 3/3 | $0.32 ($0.23–$0.37) | 2 | 0 (0–2) |

- Los rangos se solapan en las tres katas: ningún efecto establecido, como se
  esperaba. `adoption` (con `disable-model-invocation`) no se cargó en ninguna
  corrida, el hook `SessionStart` no tiene salida sin la variable y no hubo
  bloqueos de hooks.
- El revisor detectó las cuatro violaciones plantadas (HEX-7, ERR-2, RS-2,
  TDD-1) en las 6 corridas.
- Sin corridas inválidas ni 429. Costo total: $8.84.
- Hallazgo del runner: `plugins` en el JSON registra las versiones del clon
  también con `--harness-ref` (el control figura con `craft-backend`
  1.0.0-rc.2). La fuente real está en `harness_root` de cada corrida. Queda
  para otro encargo.

### Candidata 1.0.0-rc.1 (0.22.0 contra 0.20.0)

`main` en `265f747` (0.22.0) contra `63dfe77` (0.20.0) con `--harness-ref`. Entre
las dos solo están HEX-3 (un test) y WEB-2/AS-2 (plantilla Astro, que estas katas
no usan): ninguna skill, hook ni subagente cambió.

| Kata | Configuración | Aceptación | Costo | Turnos |
|---|---|---|---|---|
| rust-rename-community | main | 3/3 | $0.68 ($0.63–$0.72) | 55 (55–71) |
| rust-rename-community | control | 3/3 | $0.70 ($0.64–$0.72) | 54 (52–64) |
| rust-community-by-slug | main | 3/3 | $0.47 ($0.43–$0.52) | 54 (52–58) |
| rust-community-by-slug | control | 3/3 | $0.50 ($0.50–$0.52) | 61 (61–63) |
| review-planted-subagent | main | 3/3 | $0.25 ($0.24–$0.27) | 2 |
| review-planted-subagent | control | 3/3 | $0.20 ($0.19–$0.24) | 2 |

- Los rangos se solapan en las tres katas (en la revisión, solo en $0.24): no
  hay efecto establecido, como se esperaba sin cambios en lo que carga la sesión.
- **Desviación de la ventana:** el setup (`cargo build`) de una corrida de `main`
  falló antes de guardar su JSON, así que la retoma repitió sus 6 corridas de
  Rust (17:13–17:25, el control corrió 15:29–17:08) y la revisión de `main`
  corrió a las 20:24 (el control, 17:09). Una corrida más la cortó un 429 y se
  repitió. Costo total de las katas: $8.43.
- Hallazgo del runner: si una corrida lanza en el setup, el proceso termina sin
  guardar las válidas de esa configuración. Queda para otro encargo.

### Skill `adoption` y hook `pre-edit` (0.20.0, control intercalado)

Primera medición con `--harness-ref`: la rama del PR (`craft-backend` 0.17.0,
sin publicar) contra `main` (0.16.0, `7039347`), con la misma plantilla. Las
dos configuraciones corrieron en paralelo, kata por kata, en la misma ventana.
Sonnet implementa y Opus revisa como subagente, como en la línea base de 0.14.

| Kata | Configuración | Aceptación | Costo | Turnos | Archivos leídos |
|---|---|---|---|---|---|
| rust-rename-community | rama | 3/3 | $0.70 ($0.66–$0.71) | 63 (53–74) | 14 (1–29) |
| rust-rename-community | control | 3/3 | $0.75 ($0.72–$0.82) | 68 (65–76) | 21 (7–25) |
| rust-community-by-slug | rama | 3/3 | $0.51 ($0.47–$0.52) | 58 (49–63) | 16 (11–24) |
| rust-community-by-slug | control | 3/3 | $0.44 ($0.42–$0.50) | 54 (52–59) | 22 (19–23) |
| review-planted-subagent | rama | 3/3 | $0.30 ($0.26–$0.30) | 2 | 2 (0–2) |
| review-planted-subagent | control | 3/3 | $0.23 ($0.22–$0.25) | 2 | 0 |

- **Corrección:** igual en las dos (3/3 en las tres katas; el revisor detectó
  las 4 violaciones plantadas en todas las corridas).
- **La skill no se carga:** `adoption` (con `disable-model-invocation`) no
  aparece en ninguna corrida, y el hook `pre-edit` no bloqueó nada.
- **Costo:** los rangos no se solapan en el renombre (rama −$0.05) ni en la
  revisión (rama +$0.07), y en direcciones opuestas. Por la regla de 3 corridas
  cuentan como efectos, pero el diff no les da mecanismo: en estas sesiones el
  PR solo agrega un hook sin tokens y cambia texto de `rust.md` que nadie leyó.
  La diferencia de la revisión viene de dos corridas en que el subagente leyó
  `rust-stack/SKILL.md` (sin cambios en el PR) y archivos de dominio. No se
  actúa sobre ellos; si se repiten en la próxima ronda, se investigan.
- **Corridas inválidas:** el límite de uso cortó con 429 la consulta por slug y
  la revisión de la primera ventana (2026-11-20 23:32). Se repitieron con la
  misma etiqueta el 2026-11-22, de nuevo las dos configuraciones en paralelo.
  El renombre quedó de la primera ventana.

### Índice de ejemplos (archivado)

`references/exemplars.md` en `rust-stack` y `astro-stack`, con los `SKILL.md`
remitiendo a él. Opus:

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| astro-services-page | 0.11.0-indice | claude-opus-5-5 | 3/3 | $0.42 ($0.36–$0.48) | 23 (19–25) | 12 (10–14) | 24 (20–28) | 6k (5k–6k) | 347k (266k–443k) | 30k (26k–33k) | 22k (18k–29k) | 4 (1–4) | 1 | 6 (3–6) |
| review-planted | 0.11.0-indice | claude-opus-5-5 | 2/3 | $0.27 ($0.25–$0.36) | 12 (12–16) | 8 (7–8) | 16 (14–16) | 5k (5k–5k) | 103k (82k–105k) | 18k (17k–30k) | 34k (32k–48k) | 3 (3–6) | 0 | 0 |
| rust-community-by-slug | 0.11.0-indice | claude-opus-5-5 | 3/3 | $1.16 ($1.03–$1.21) | 64 (63–69) | 20 (19–22) | 40 (38–44) | 19k (18k–19k) | 987k (838k–1219k) | 73k (63k–75k) | 85k (66k–92k) | 22 (21–23) | 1 | 0 |
| rust-rename-community | 0.11.0-indice | claude-opus-5-5 | 3/3 | $1.50 ($1.49–$1.51) | 81 (80–87) | 31 (25–32) | 62 (50–64) | 26k (26k–28k) | 1851k (1458k–1893k) | 77k (77k–80k) | 79k (77k–79k) | 23 (23–25) | 0 | 0 (0–1) |

- La aceptación de las katas de implementación no cambia (9/9).
- No hay ahorro: la línea base ya leía 0 archivos del harness.
- En `rust-rename-community`, el costo subió de $1.35–1.37 a $1.49–1.51, con
  rangos que no se solapan. Ninguna de esas corridas leyó el índice. Pero el
  cambio también modificó el texto del `SKILL.md` de `rust-stack`, que el
  agente carga siempre. No se publica hasta entender la diferencia.
- **Archivado.** No se investiga más. En una kata el proyecto es la plantilla,
  así que el índice no tiene nada que ahorrar, y la subida del renombre cae
  dentro de la variación entre días: el control sin índice del 2026-11-13 dio
  $1.33–1.63. Se retoma cuando exista una kata sobre un proyecto que se apartó
  de la plantilla.

### Dónde crece el contexto

Investigación sobre los transcripts de `rust-rename-community`, `0.11.0-base`
contra `0.11.0-indice`, sin corridas nuevas.

**Precios implícitos.** Un ajuste lineal del costo de las 24 corridas de Opus
contra sus tokens es exacto (error de 1e-15). Por millón de tokens: entrada
$4, caché leída $0.20, caché escrita $8 y salida $20. Las escrituras en caché
de Claude Code son de 1 hora (`ephemeral_1h_input_tokens`).

**Descomposición del costo** (media de las 3 corridas):

| | Caché leída | Caché escrita | Salida | Total |
|---|---|---|---|---|
| base | $0.25 | $0.63 | $0.48 | $1.36 |
| índice | $0.35 | $0.62 | $0.53 | $1.50 |

La caché escrita, que es el contenido nuevo de cada petición (sobre todo
resultados de herramientas), pesa casi la mitad del costo y no cambió. La
diferencia se reparte en un 70 % de caché leída y un 30 % de salida.

**El contexto por petición no creció.** Los "15.000 a 23.000 tokens por
turno" salen de dividir la caché leída por `num_turns`, pero `num_turns` no
cuenta peticiones a la API: una respuesta con seis lecturas en paralelo es una
petición y seis turnos. Por petición, el contexto medio es de unos 60k en la
base y unos 62k con el índice. Después de cargar las skills, el texto nuevo
del `SKILL.md` de `rust-stack` agrega entre 90 y 200 tokens por petición
(28.0–28.5k contra 28.0–28.6k), alrededor de $0.001 por corrida.

**Lo que cambió es la cantidad de peticiones y de ediciones:**

| | Peticiones | `Edit` | Salida | Resultados de herramientas |
|---|---|---|---|---|
| base | 20, 21, 25 | 20, 20, 18 | 24.3k, 24.4k, 23.9k | 86k caracteres (81k–91k) |
| índice | 32, 25, 31 | 24, 24, 25 | 25.7k, 28.0k, 25.6k | 79k caracteres (77k–79k) |

- **Secuencias:** coinciden hasta terminar de leer (skills y luego 4 o 5
  rondas de 5 a 12 lecturas en paralelo). Divergen en la implementación. La
  base edita en lotes grandes (`Edit`×5 + `Bash`, y luego `Edit` + `Write`×7);
  el índice, en lotes de 2 a 3 `Edit` intercalados con `Bash`. Las peticiones
  con una sola herramienta pasan de 3, 6 y 13 a 18, 6 y 15.
- **Rangos:** las ediciones y la salida no se solapan entre configuraciones.
  Las peticiones sí (máximo 25 en la base, mínimo 25 con el índice).
- **Descartado:** los resultados de herramientas fueron *menores* con el
  índice (lee menos: 54k contra 66k caracteres de `Read`). Ningún hook bloqueó
  de forma que explique la diferencia (0 bloqueos en la base; 0, 0 y 1 con el
  índice). La versión de Claude Code fue la misma (2.1.316).

**Hipótesis que quedan** (no concluyente):

1. El texto nuevo del `SKILL.md` ("lee el ejemplo equivalente del índice y
   copia su forma"; "no explores `template/`") cambió la estrategia de
   implementación hacia ediciones más pequeñas y más idas y vueltas. Es
   compatible con que el índice no se leyera en ninguna corrida del renombre.
2. Variación del servicio en el tiempo: las dos configuraciones corrieron con
   5 horas de diferencia (16:59 y 21:46), sin otra variable de por medio.

Para separarlas hay que correr las dos configuraciones intercaladas, en la
misma ventana de tiempo. La métrica nueva (peticiones y caracteres de
resultados por herramienta, en la tabla y en `tool_result_chars` del JSON)
hace visible esta clase de diferencia en adelante.

### Mapa del proyecto en la plantilla Rust (revertido)

`stacks/rust/template/CLAUDE.md`, de 267 palabras: capas, el recorrido de
`create_community` con sus rutas y los comandos. Opus, contra la [línea
base](#línea-base) y contra `0.12.0-diffs-opus` (el mismo harness sin el
mapa, corrido el mismo día):

| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida | Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness | Bloqueos de hooks |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rust-community-by-slug | 0.12.0-mapa | claude-opus-5-5 | 3/3 | $1.19 ($1.12–$1.20) | 50 (49–56) | 22 (16–24) | 44 (32–48) | 18k (18k–19k) | 1290k (820k–1319k) | 70k (69k–76k) | 81k (77k–98k) | 6 (4–23) | 0 | 0 |
| rust-rename-community | 0.12.0-mapa | claude-opus-5-5 | 3/3 | $2.03 ($1.68–$2.09) | 89 (75–91) | 36 (32–42) | 64 (54–80) | 33k (26k–33k) | 2330k (1728k–2874k) | 97k (85k–97k) | 139k (130k–141k) | 25 (24–29) | 0 | 0 (0–1) |

- **Aceptación:** se mantiene, 6/6.
- **`rust-community-by-slug`:** los archivos leídos bajan a una mediana de
  6, pero con un rango de 4 a 23 que se solapa con el de la base (20–21). El
  costo no cambia ($1.12–1.20 contra $1.06–1.22).
- **`rust-rename-community`:** el costo **sube** a $1.68–2.09, contra
  $1.35–1.37 en la base y $1.33–1.63 en el control. Los rangos no se
  solapan.
- **Por qué sube:** con el mapa, la sesión principal invocó a
  `architecture-reviewer` en las 3 corridas (Sonnet, $0.08–0.12) y siguió
  trabajando sobre sus hallazgos. Hubo más `Bash` (12–18 contra 8–10), más
  `Edit` (22–29 contra 18–20) y más peticiones (32–42 contra 20–27). En el
  control, ninguna corrida lo invocó. El mapa no menciona al revisor.
- **Decisión:** no baja ni los archivos ni el costo con rangos separados, así
  que se revierte (regla de D).
- **El aumento no viene del mapa en sí,** que son 267 palabras en el
  contexto, sino de que la sesión invocó al revisor y siguió trabajando sobre
  sus hallazgos. Es un **cambio de flujo, no una regresión**: implementar y
  revisar cuesta más que solo implementar.
- **Pendiente:** si el flujo debe incluir al revisor se decide con la medición
  del revisor como subagente.
