# Shell Shock Tool — Green Version

**SST Green** es una línea experimental de SST para herramientas nativas con vistas a pantalla completa. No sustituye ni actualiza la línea **Blue**.

## Estado actual

- Consola Windows Rust + Slint, con el área de terminal a ancho completo y la isla SST original.
- Vista nativa de herramienta al invocar `triage` en la interfaz interactiva; admite desplazamiento y vuelve a la consola con `Esc` o `Q`.
- `net monitor`, `sys top` y `sigsour` utilizan sus pantallas alternativas de terminal; reciben un fondo dedicado en modo herramienta.
- Salidas de comandos y datos de sistema reales: no se inventan métricas ni comandos.
- Tema industrial oscuro/crema/naranja para vistas de herramienta, con colores semánticos de estado.

## Aislamiento

El código fue importado de la línea Blue únicamente como base de desarrollo. Se excluyeron disparadores de publicación y workflows de Blue. La versión local es `0.0.greenversion.2`; el comparador del actualizador rechaza cambios entre canales.

**Compilación local Windows:** `cargo build --release` desde este repositorio. No hay una compilación Windows verificada para Green todavía.

