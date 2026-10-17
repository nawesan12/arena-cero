# Arena Cero

FPS táctico de mundo abierto, portado de una versión web (Three.js) a **Rust + Bevy 0.20**.

- **Distrito Sur**: ciudad procedural de 330 m con siete puestos enemigos. Sigilo o combate abierto;
  cada objetivo cambia el mapa (sin antena no hay refuerzos, sin energía se apagan las torretas…).
- **Oleadas**: cuatro arenas con drones, kamikazes, pesados y el Coloso.
- Movimiento rápido: deslizarse, doble salto, dash y gancho. Cuatro armas, granadas, mejoras.
- Todo procedural: mapas, modelos, texturas, efectos y música se generan por código.

## Jugar

Descargá el ejecutable de tu sistema desde la pestaña **Actions** (último build verde → *Artifacts*),
o compilalo:

```sh
cargo run -p arena_client --release                 # Distrito Sur
cargo run -p arena_client --release -- --map deposito   # también fundicion, cantera, azotea
```

Requiere Rust estable ≥ 1.97. En Linux: `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`.

Controles: WASD, Shift correr, C agacharse/deslizar, Espacio saltar, Q dash, E/Ctrl gancho,
clic izq. disparar, clic der. apuntar, 1-4 armas, R recargar, G granada, F usar, T marcar,
Tab mapa, F1-F3 calidad, F4 FPS, M música, Esc pausa.

## Estructura

- `crates/sim`: simulación sin gráficos (mundo, IA, armas, objetivos). Determinista; la usará el servidor.
- `crates/client`: cliente Bevy (render, interfaz, audio sintetizado).
- `referencia-web/`: la versión web original, que es la especificación del port.

Más detalle en [`CLAUDE.md`](CLAUDE.md).
