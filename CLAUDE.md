# Arena Cero — guía para portar a nativo

FPS táctico de mundo abierto. La versión de referencia es web (Three.js r128) y está en `referencia-web/`:
`arena-cero.html` se abre con doble click y `src/` tiene el código partido por sistema (se une con `build.py`).
Esta carpeta es el punto de partida para rehacerlo en un motor nativo. **El código web es la especificación**: los valores, la IA y el diseño del mapa ya están probados; hay que traducirlos, no reinventarlos.

## Motor elegido: Rust + Bevy 0.20 (decidido el 17/10/2026)
Toolchain: Rust estable ≥ 1.97.1 (Bevy 0.20 lo exige). Workspace de Cargo:

- `crates/sim` (`arena_sim`): **simulación pura, sin Bevy** (solo `glam`). Mundo de cajas AABB en grilla de 8 m
  (`world.rs`), generadores de mapas (`maps.rs`, `city.rs`), movimiento del jugador (`player.rs`), RNG mulberry32
  idéntico al de la web (`rng.rs`). Todo determinista: servidor y clientes arman el mismo mapa con la misma semilla.
  Corre a paso fijo de 60 Hz (`TICK`). Acá va toda la lógica de juego (armas, IA, objetivos) para que el servidor
  la ejecute con autoridad.
- `crates/client` (`arena_client`, binario `arena-cero`): Bevy. Texturas procedurales (`textures.rs`), materiales
  (`materials.rs`), mallas fundidas por material y zona de 88 m (`world_render.rs`), input + paso fijo + cámara
  interpolada (`player.rs`), HUD (`hud.rs`).
- Más adelante: `crates/server` (headless, autoritativo) y `crates/net` (protocolo).

Comandos:
- `cargo test -p arena_sim` — tests de la simulación.
- `cargo run -p arena_client -- --map ciudad` (también deposito, fundicion, cantera, azotea).
- `cargo run -p arena_client -- --at x,z,yaw,pitch --weapon 1-4 --shot captura.png` — captura a los 3 s y sale.
  Ojo: si la ventana queda tapada, la captura sale negra (no es un bug del render).

Reglas del port:
- Valores, orden de llamadas al RNG y diseño, tal cual la web. Si se corrige un bug de la web, dejarlo comentado.
- La simulación no depende de la tasa de cuadros: entrada → `step(dt = TICK)` → eventos. El cliente solo dibuja.
- Las pulsaciones (saltar, dash, gancho) se guardan hasta el próximo paso fijo para no perderlas.

## Online por etapas
1. **Servicios**: cuentas, ranking validado en el servidor (hoy la web confía en el puntaje del cliente), guardado.
2. **Cooperativo 2–4**: servidor autoritativo con la IA; los clientes mandan `PlayerInput` y predicen su propio
   movimiento con el mismo `arena_sim`; reconciliación con el estado del servidor.
3. **PvP**: compensación de lag para los disparos y anti-trampas.

## Orden del port
Hecho (detalle en `docs/`): todo el port de la web (mundo, 5 mapas, jugador, armas, audio, enemigos con A*,
objetivos, F/T/Tab, mejoras, HUD) y además vehículos, ciudad de 590 m con barrios y campamentos, patrullas en auto,
misiones dinámicas, economía/tienda/gadgets, día y noche, Ciudad 2.0 (cajas giradas, plaza, bulevar curvo,
diagonal, rotondas, locales, subte con 3 estaciones), peatones y tránsito, gancho-péndulo,
la **Campaña de operaciones** (`campaign.rs` + `campaign_ui.rs`, 10 operaciones con briefing, menú principal,
rango), la **Ciudad 3.0** porteña (`town.rs`, `props.rs`, `city_edge.rs`), personas con esqueleto (`humanoid.rs`)
y sonido de ciudad (`ambience.rs`).
Falta: sensación arcade (bloom, hit-stop, combo), variedad visual de la ciudad, oleadas con cambio de mapa,
servidor y cooperativo. Plan completo en `docs/HISTORIAL.md` ("Lo que sigue").
- `cargo run -p arena_client -- --op 3` arranca directo en una operación; `--libre`, en el mundo libre.

1. **Jugador** (`18-ciclo-de-juego.js`, `21-gancho...js`): CharacterBody3D con correr, agacharse/deslizar, doble salto, dash (Q) y gancho (Ctrl/E). Valores: velocidad 7 (10 corriendo), salto 8.2, gravedad 24, gancho 55 m con tirón 58 y tope 34 m/s, dash 24 m/s 0,16 s cada 3 s.
2. **Armas** (`04-armas.js`, `16-disparo...js`, `06-materiales...js`): rifle, escopeta (9 perdigones), riel (atraviesa), lanzagranadas. Apuntado con click derecho, retroceso, críticos al núcleo/cabeza ×2.
3. **Ciudad** (`03-mapas.js`, `03b-ciudad.js`): generador procedural con semilla fija. Grilla 7×7 de manzanas de 32 m, calles de 12 m, muro a ±164 m. Usar los mismos helpers: `addBuilding` (pisos de 3,4 m, ventanas, puertas, escaleras alternadas, tabiques, balcones, escalera de incendio), `addTower`, `addHall`, `addSkybridge`, `addHighway`, `addFrame` (obra), contenedores, autos. En Godot: MeshInstance3D con MultiMesh o fusión con SurfaceTool + CollisionShape3D por caja; bakear NavigationMesh al generar.
4. **Enemigos** (`15b-tactico.js`): soldados (patrulla, búsqueda, combate con cobertura, granadas, francotirador con láser, comandante con escudo), torretas con láser, drones de vigilancia y refuerzos. Detección gradual por visión (40 m, cono 65°, agachado ×0,5). Reglas de dificultad que ya se ajustaron: reacción 0,8–1,6 s cada vez que te vuelven a ver, máximo 2 tiradores por puesto, precisión base 0,4 (−0,009 por metro), daño 4.
   Los soldados de la web no tienen pathfinding (caminan en línea recta y se traban): acá van con navegación.
5. **Objetivos y mundo abierto** (`17b-ciudad-juego.js`): 7 puestos con efectos globales (antena = sin refuerzos, subestación = torretas apagadas, oficinas = enemigos marcados, arsenal = armas, obra = +1 intento, trenes = sin drones, mando = comandante). Interacción con F, derribo por la espalda, marcar con T, mapa con Tab, 3 intentos, reabastecimiento en puestos despejados, mejoras al completar objetivos.
6. **Interfaz y audio** (`19-hud...js`, `07-audio.js`, `08-musica...js`): HUD, marcadores en pantalla, radar, música dinámica por nivel de alerta.
7. Modo oleadas (`17-oleadas.js`) como extra.

## Hoja de ruta después del port
Noche y linterna · gadgets (humo, señuelo, dron explorador) · economía y tienda en el refugio · árbol de habilidades · guardado · vehículos · clima · más roles enemigos (médico, ingeniero, perro) · contraataques · jefe final · interiores detallados y destrucción · cooperativo 2–4.

## Documentación

Ver `docs/JUGABILIDAD.md`, `docs/ARQUITECTURA.md` y `docs/HISTORIAL.md`. Mantenelas al día con cada cambio.
