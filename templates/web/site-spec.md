# Spec del sitio — {Cliente}

> Estado: borrador | aprobado (fecha) · Versión: 1
> Fuente: cuestionario de descubrimiento del {fecha}. Este documento es el contrato
> funcional: lo que no está aquí no está en el alcance.

## Objetivo

- **Acción principal:** {contactar | cotizar | …}
- **Indicador de éxito:** {p. ej. 10 mensajes válidos al mes a los 6 meses}
- **Público:** {quién, qué busca}

## Páginas

| Clave (`routes.ts`) | URL | Propósito | Contenido aportado por | Estado del contenido |
|---------------------|-----|-----------|------------------------|----------------------|
| `home` | `/` | | | pendiente |
| `about` | `/sobre/` | | | pendiente |
| `contact` | `/contacto/` | Formulario | plantilla | listo |
| `privacy` | `/privacidad/` | Aviso de privacidad | cliente + plantilla | pendiente de revisión legal |

## Contenido estructurado

{Colecciones con su schema: productos, artículos, servicios. Por cada campo: tipo,
obligatorio, quién lo llena.}

## Marca

- Logo: {archivos}
- Colores (`@theme`), con el contraste verificado si se usan como texto: {…}
- Tipografías (servidas desde el propio dominio): {…}
- Tono: {…}

## Formulario

- Destino: {correo}
- Campos: nombre, correo, mensaje {+ asunto, si aplica}
- Antispam: campo trampa + tiempo + límite por IP {+ Turnstile, si se justifica}

## Técnica

- Stack: plantilla `stacks/astro` (Astro + Cloudflare Workers + Tailwind 4)
- Dominio y DNS: {registrador, quién tiene el acceso}
- Redirecciones 301 desde el sitio anterior: {lista}
- Analítica: {Cloudflare Web Analytics | ninguna}
- Idiomas: {es}

## Barra de calidad (no negociable)

WEB-1 a WEB-10: accesibilidad = 100 con axe en cada página, rendimiento ≥ 90, CSP sin
`unsafe-inline`, sin terceros en el navegador, formulario que funciona sin JS.

## Fuera de alcance

{Lo que se habló y no entra: tienda en línea, área de clientes, blog con editor visual…}

## Preguntas abiertas

| # | Pregunta | Responsable | Fecha límite |
|---|----------|-------------|--------------|
