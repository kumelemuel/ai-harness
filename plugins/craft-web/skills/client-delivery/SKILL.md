---
name: client-delivery
description: Proceso comercial para sitios web de clientes, de punta a punta. Cubre el cuestionario de descubrimiento, la spec del sitio, la cotización, la construcción con la plantilla y el checklist de entrega. Usar cuando se inicia, cotiza, planifica o entrega un sitio para un cliente, cuando hay que preparar una reunión de descubrimiento, o cuando se pregunta qué falta para publicar.
---

# Entrega de sitios a clientes

Plantillas en `references/`:

| Paso | Archivo | Resultado |
|------|---------|-----------|
| 1. Descubrimiento | `discovery.md` | Respuestas del cliente; lo desconocido queda como pregunta abierta |
| 2. Spec | `site-spec.md` | Contrato funcional aprobado: páginas, contenido, marca, alcance y lo que queda fuera |
| 3. Cotización | `quote.md` | Horas por bloque a partir de la spec |
| 4. Construcción | skill `astro-stack` | Sitio desde la plantilla, con la barra WEB-1 a WEB-10 |
| 5. Entrega | `delivery-checklist.md` | Cada ítem con evidencia |

## Reglas

- **No construyas sin una spec aprobada.** Lo que no está en la spec no está en el alcance.
- **Las páginas de la spec son las claves de `src/routes.ts`.** Úsalas para crear la
  tabla de rutas.
- **No supongas contenido del cliente.** Un texto, una foto o un dato legal que falta
  queda como pregunta abierta en la spec. Nunca se publica texto de relleno.
- **La barra de calidad no se negocia ni se cotiza aparte:** viene incluida en la plantilla.
- **La entrega no termina sin un envío real del formulario en producción,** recibido en
  el buzón del cliente.
- Después de cada proyecto, compara las horas reales con `quote.md` y ajusta la tabla.

## Al preparar la spec

1. Lee las respuestas del cuestionario.
2. Llena `site-spec.md` con lo que se sabe. Cada vacío se convierte en una fila de
   "Preguntas abiertas", con su responsable.
3. Propón la tabla de rutas (clave → URL) y los schemas de las colecciones.
4. Lista explícitamente lo que queda fuera de alcance.
