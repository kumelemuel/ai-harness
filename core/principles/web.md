# Principios — Sitios web informativos

> **Estado:** v0.1
> **Origen:** destilado de kumelemuel.dev (Astro + Vercel, es/en, tests e2e con axe y presupuestos de Lighthouse) y kidus-akaal.com (Astro + Cloudflare, catálogo, Turnstile y rate limit).
> **Relacionados:** [verification.md](verification.md) (`VER`), [tdd.md](tdd.md) (`TDD`).

Formato de cada principio: ver [README](README.md).

El objetivo de estos sitios no es la pureza arquitectónica sino **entregar rápido,
de forma repetible y con una barra de calidad que no dependa de acordarse**.
Cada principio tiene una verificación automática salvo donde se indica.

---

## WEB-1 — Estático por defecto; servidor solo donde es imprescindible

**Regla.** Toda página se genera en el build. Solo corre en el servidor lo que no puede existir sin él: el envío de un formulario, una página de confirmación sin JS. Cada ruta dinámica se declara explícitamente.

**Fundamento.** Una página estática es la más rápida, la más barata de servir, la que no se cae y la que no expone superficie de ataque. Un sitio informativo casi no necesita servidor.

**Violaciones típicas.** Activar renderizado en servidor para todo el sitio "por si acaso". Obtener en el cliente datos que se conocen en el build.

**Verificación.** El build lista las rutas dinámicas; un test afirma que son exactamente las esperadas.

---

## WEB-2 — Una sola fuente de rutas

**Regla.** Las URLs del sitio se definen en un solo módulo (clave → ruta). Menú, enlaces internos, `canonical`, `hreflang`, sitemap y tests las obtienen de ahí. Nunca se escribe una ruta interna a mano en el markup. Renombrar una URL publicada exige una redirección 301 desde la anterior.

**Fundamento.** Una URL duplicada a mano termina rota en algún lugar. Con una fuente única, cambiar un slug es un cambio de una línea más su redirección, y los tests detectan una página sin archivo o un enlace sin ruta.

**Violaciones típicas.** `href="/contacto/"` escrito en un componente. Renombrar una página sin redirigir la URL vieja (se pierde el posicionamiento ganado).

**Verificación.** Test unitario: cada ruta declarada tiene su archivo de página. Test e2e: cada redirección responde 301 hacia su destino.

---

## WEB-3 — Contenido como datos tipados, separado de la presentación

**Regla.** El contenido (textos, productos, artículos, datos de la marca) vive en colecciones o módulos de datos con schema, no incrustado en el markup. Los componentes reciben datos y los presentan. Lo que se repite en varias páginas (nombre de la marca, contacto, redes) existe una sola vez.

**Fundamento.** El cliente cambia textos y productos más que el diseño. Si el contenido es dato validado, un cambio no rompe el sitio y el build falla si falta un campo obligatorio (descripción SEO, texto alternativo).

**Violaciones típicas.** El precio o el correo de contacto copiado en cinco componentes. Un artículo sin `description` que se publica igual.

**Verificación.** Schemas de las colecciones (el build falla); `astro check` o equivalente.

---

## WEB-4 — Accesibilidad WCAG 2.2 AA verificada en cada página

**Regla.** Cada página pasa una auditoría automática de WCAG 2.2 AA sin violaciones, tiene un único `<h1>`, `lang` correcto, enlace para saltar al contenido, foco visible y navegación por teclado. Los colores de marca se validan por contraste antes de usarse como texto.

**Fundamento.** La accesibilidad es una barra, no una aspiración: si no se verifica, se pierde en el próximo cambio de diseño. También es SEO y es legal en varios mercados.

**Violaciones típicas.** Un color de marca usado como texto sin el contraste suficiente. Íconos sin nombre accesible. Un `<h1>` por sección. Animaciones que ignoran `prefers-reduced-motion`.

**Verificación.** Test e2e con axe (etiquetas `wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa`, `wcag22aa`) en todas las páginas, después de desplazarse hasta el final. Lighthouse accesibilidad = 100. La calidad de los textos alternativos queda a revisión.

---

## WEB-5 — Presupuestos de rendimiento que fallan el build

**Regla.** Se fijan presupuestos y se verifican en CI sobre el build de producción: rendimiento ≥ 90, accesibilidad = 100, buenas prácticas ≥ 95, SEO ≥ 90, CLS ≤ 0,1. Las imágenes se optimizan en el build, con dimensiones declaradas. Las fuentes se sirven desde el propio dominio. No hay JavaScript que no aporte.

**Fundamento.** El rendimiento se degrada de a poco, con cada imagen y cada script. Un presupuesto que falla convierte esa degradación en una decisión explícita.

**Violaciones típicas.** Una foto de 4 MB en el hero. Una librería de animación para un efecto que resuelve CSS. Una fuente cargada desde un CDN de terceros.

**Verificación.** Lighthouse CI con aserciones como `error`.

---

## WEB-6 — CSP estricta, sin `unsafe-inline`

**Regla.** Toda página envía una Content Security Policy que solo permite el propio origen, más los hashes de los scripts y estilos que el build genera. `unsafe-inline` y `unsafe-eval` están prohibidos. Cada origen externo permitido (p. ej. un captcha) se justifica en un comentario. Se envían también HSTS, `nosniff`, `Referrer-Policy`, `Permissions-Policy` y protección contra *framing*.

**Fundamento.** La CSP es la defensa contra inyección de scripts. Una política con `unsafe-inline` no defiende contra lo principal que debería defender.

**Violaciones típicas.** Agregar `unsafe-inline` para que funcione un script de terceros. Un `onclick=""` o un `style=""` en el markup que la política bloquea.

**Verificación.** Test e2e: ninguna página produce violaciones de CSP ni errores de JS, contra el build servido con sus cabeceras reales. Test que afirma que la política no contiene `unsafe-inline`.

---

## WEB-7 — Privacidad por defecto

**Regla.** Nada de terceros en el navegador salvo lo imprescindible y declarado: las fuentes se descargan en el build, la analítica no usa cookies y no hay rastreadores publicitarios. El aviso de privacidad describe exactamente lo que el sitio hace (formulario, analítica, proveedores) y se actualiza cuando eso cambia. El sitio no necesita un banner de cookies.

**Fundamento.** Cada tercero en el navegador es una filtración de datos del visitante, un riesgo de rendimiento y una obligación legal. Un sitio informativo puede no tener ninguno.

**Violaciones típicas.** Google Fonts cargado en runtime. Un píxel de seguimiento agregado "solo para probar". Un aviso de privacidad genérico copiado que no coincide con lo que el sitio hace.

**Verificación.** Test e2e: todas las solicitudes de red de cada página van al propio origen o a una lista permitida. La coherencia del aviso queda a revisión.

---

## WEB-8 — Formularios robustos: sin JS, antispam en capas, reglas puras

**Regla.**

- El formulario funciona sin JavaScript (envío HTML con página de confirmación). El JavaScript solo mejora la experiencia.
- La validación ocurre en el servidor y se refleja en el cliente con mensajes por campo.
- El antispam va en capas: campo trampa, tiempo mínimo de llenado, límite de envíos por IP y, si el volumen lo justifica, un captcha. A un bot se le responde con un éxito falso.
- **Las reglas (validación, antispam, armado del correo) viven en un módulo puro, sin dependencias del framework ni del proveedor, con tests unitarios.** El handler solo orquesta: parsea, aplica las reglas, envía y responde. Es la versión liviana de HEX-2 y HEX-7.
- Todo dato del visitante se escapa antes de ir en un correo HTML. El asunto no admite saltos de línea.

**Fundamento.** El formulario es la única parte del sitio que recibe datos de desconocidos y la que el cliente usa para medir si el sitio "funciona". Si sus reglas están en el handler, no se pueden probar sin enviar correos reales.

**Violaciones típicas.** Validación solo en el cliente. Un formulario que requiere JS. Reglas mezcladas con la llamada al proveedor de correo. Responder con error a un bot (lo hace reintentar).

**Verificación.** Tests unitarios de las reglas. Test e2e del formulario con y sin JS por la vía del campo trampa (nunca envía correo real).

---

## WEB-9 — SEO técnico completo y verificado

**Regla.** Cada página indexable tiene título único, descripción de 140 a 160 caracteres, `canonical` absoluto, Open Graph con imagen y JSON-LD acorde a su tipo (`Organization`/`Person` y `WebSite` en todo el sitio; `Article`, `Product`, `BreadcrumbList` donde corresponda). Las URLs terminan de forma consistente (con o sin `/`, nunca ambas). El sitemap excluye páginas `noindex` y secciones vacías. `robots.txt` apunta al sitemap.

**Fundamento.** En un sitio informativo, el SEO técnico es la mitad del valor que se entrega al cliente, y es casi todo automatizable.

**Violaciones típicas.** El mismo `<title>` en todas las páginas. Página de confirmación del formulario indexada. JSON-LD que no coincide con el contenido visible.

**Verificación.** Test e2e de metadatos por página; test del sitemap y de `robots.txt`. Lighthouse SEO ≥ 90. La coherencia del JSON-LD con el contenido queda a revisión.

---

## WEB-10 — Secretos solo en el servidor, declarados en un schema

**Regla.** Toda variable de entorno se declara en un schema con su contexto (`server`/`client`) y su acceso (`secret`/`public`). Los secretos nunca llegan al cliente ni al repositorio. El repositorio incluye un `.env.example` sin valores.

**Fundamento.** Un schema hace que el build falle si falta una variable, y que el framework impida importar un secreto desde código de cliente.

**Violaciones típicas.** Una API key con prefijo público. Un `.env` versionado. Leer `process.env` directamente.

**Verificación.** El schema de entorno del framework (falla el build). `.gitignore` incluye `.env*` salvo `.env.example`.

---

## Antipatrones

| En vez de | Usar |
|-----------|------|
| Renderizado en servidor para todo el sitio | Estático; servidor solo en rutas declaradas (WEB-1) |
| Rutas internas escritas a mano | Módulo único de rutas (WEB-2) |
| Renombrar una URL sin redirección | 301 desde la anterior (WEB-2) |
| Contenido incrustado en el markup | Colecciones y datos con schema (WEB-3) |
| "Lo revisamos a ojo" | axe + Lighthouse en CI (WEB-4, WEB-5) |
| `unsafe-inline` en la CSP | Hashes generados por el build (WEB-6) |
| Fuentes o scripts de terceros en runtime | Servidos desde el propio dominio (WEB-7) |
| Formulario que requiere JS | Envío HTML + mejora progresiva (WEB-8) |
| Reglas del formulario dentro del handler | Módulo puro con tests (WEB-8) |
| `process.env` suelto | Schema de entorno (WEB-10) |

---

## Decisiones abiertas

1. **Captcha por defecto.** ¿Se activa en todo sitio de cliente o solo cuando aparece spam? Hoy: campo trampa + tiempo + límite por IP siempre; captcha opcional.
2. **Multi-idioma.** Cuándo se paga la complejidad de i18n desde el inicio y cuándo se agrega después.
