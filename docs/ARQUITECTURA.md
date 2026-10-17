# Arena Cero — Arquitectura

Rust (estable ≥ 1.97) + Bevy 0.20. Unas 14.500 líneas en dos crates.

```
arena-cero/
├── crates/sim       arena_sim: la simulación, sin gráficos (solo glam)
├── crates/client    arena_client: el juego (Bevy), binario `arena-cero`
├── referencia-web/  la versión web original (Three.js r128): la especificación
├── docs/            esta documentación
└── .github/         CI: tests + ejecutables para macOS, Windows y Linux
```

## Principio: la simulación no sabe que existe una pantalla

Todo lo que es juego (mundo, colisiones, movimiento, armas, IA, objetivos, economía, director) vive en
`arena_sim` y es **determinista**: mismas entradas y semilla, mismo resultado. Corre a paso fijo de
60 Hz (`TICK`). El cliente junta la entrada, llama a la simulación y dibuja lo que pasó. Así el mismo
código va a correr en el servidor del cooperativo, y el cliente lo va a usar para predecir.

Flujo de un paso (en `client/player.rs → step_player`, `FixedUpdate`):

1. Entrada del cuadro → `PlayerInput` (movimiento, vista, pulsaciones guardadas en `Latch`).
2. `PlayerState::step` (o `Game::drive` si estás manejando).
3. `Arsenal::step` → eventos de armas → `Game::weapon_events` (disparos contra enemigos, autos, civiles).
4. `Game::step(level, target)`: enemigos, proyectiles, granadas, gadgets, peatones, director, autos IA.
5. `Game::step_objectives`: interacción con F, cargas, antena, mejoras.
6. Los `GameEvent` resultantes se traducen a sonido, partículas, carteles, daño al jugador, etc.

## `arena_sim`

| Módulo | Qué hace |
|---|---|
| `rng.rs` | mulberry32 idéntico al `rng(seed)` de la web (misma secuencia con la misma semilla). |
| `mat.rs` | Etiquetas de material (`Mat`); el cliente decide cómo se ven. |
| `world.rs` | Mundo de cajas en grilla de 8 m: `collide_xz`, `ground_at`, `ceiling_at`, `box_at`, `ray`, `has_los`. Soporta **cajas giradas** (cada colisionador puede tener un marco local) y **huecos en el piso** (subte). Lista de piezas visuales (cajas con giro, cilindros, árboles, grupos móviles). |
| `maps.rs` | Ficha de cada mapa (`MapInfo`) y las 4 arenas (puerto de `03-mapas.js`). |
| `city.rs` | Distrito Sur (puerto exacto de `03b-ciudad.js`, semilla 20261016) + barrios exteriores (semilla 20261017), campamentos, 2.ª autopista, autos, lugares de botín, calles. |
| `city2.rs` | Ciudad 2.0: Plaza Mayor, bulevar curvo, Diagonal Norte, rotondas, locales, puestos y estaciones de subte. Submódulo de `city` (usa sus constructores). |
| `player.rs` | Movimiento del jugador y gancho-péndulo. Función pura de (estado, entrada, mundo). |
| `weapons.rs` | Armas, recarga, dispersión, retroceso, granadas (fragmentación, humo, señuelo) y su física de rebote. |
| `enemies.rs` | Drones, soldados, torretas, vigilancia: percepción (vista, cono, noche, humo), máquina de estados, cobertura, disparos con probabilidad de acierto. Zonas de golpe por rayo (esferas y cajas orientadas). |
| `nav.rs` | Grilla de navegación de 0,5 m por puesto y A* con suavizado. |
| `game.rs` | La partida: enemigos, proyectiles, granadas, explosiones, daño, muertes, puntaje, oleadas, alarmas por puesto, refuerzos, `GameEvent`. |
| `objectives.rs` | Interacción con F, derribos, cargas, efectos de los puestos, marcado, victoria. |
| `vehicle.rs` | Física de autos (bicicleta arcade, 3 círculos de choque, colisionador móvil) y piloto IA (patrulla, persecución, destrabarse, tránsito civil). |
| `director.rs` | Hora del día, patrullas en auto, misiones dinámicas, daño a vehículos. |
| `economy.rs` | Créditos, botín, tienda, gadgets (humo, señuelo, dron explorador). |
| `people.rs` | Peatones y tránsito civil. |
| `perks.rs` | Las 11 mejoras y la oferta de tres. |

Tests: 45, en cada módulo (`cargo test -p arena_sim`). Corren sin ventana.

## `arena_client`

| Módulo | Qué hace |
|---|---|
| `main.rs` | Opciones de línea de comandos, armado de la app, captura automática y modo de medición. |
| `config.rs` | Preferencias en `~/.arena-cero` (calidad, música, FPS). |
| `textures.rs` | Texturas procedurales (los mismos dibujos de canvas de la web) con mipmaps. |
| `materials.rs` | Aspecto de cada `Mat`. |
| `world_render.rs` | Cajas fundidas por (material, sombra, zona de 88 m), con giro; piso en baldosas con huecos; cilindros; árboles fundidos por zona; cielo; sol. |
| `render_scale.rs` | La escena 3D se dibuja en una textura más chica que la ventana y se estira; el HUD va encima a resolución nativa. |
| `quality.rs` | Calidad (F1-F3), tope de FPS, sombras, distancia, antialias. |
| `player.rs` | Entrada, paso fijo, eventos de la partida, cámara (primera persona interpolada; tercera persona al manejar). |
| `gun.rs`, `gunmodels.rs` | Armas en primera persona en una capa y cámara propias; modelos procedurales (perfiles extruidos con earcut) fundidos por material; animación. |
| `hook_view.rs` | Lanzador, cable, garra y anillo del gancho. |
| `enemies_view.rs` | Modelos y animación de enemigos y cadáveres; distancia de dibujo. |
| `vehicles_view.rs` | Autos, ruedas, chatarra y sonido del motor. |
| `people_view.rs` | Peatones animados. |
| `extras_view.rs` | Cajas de botín, humo, señuelos, drones exploradores, rehenes. |
| `fx.rs` | Partículas, trazadoras, luz de explosiones, granadas en vuelo. |
| `audio.rs` | Sintetizador propio (osciladores, ruido filtrado, envolventes) como fuente `Decodable` de Bevy: todos los efectos de la web y la música dinámica en 4 capas por tensión, sin archivos de sonido. |
| `hud.rs` | Interfaz en juego y pantalla de pausa. |
| `markers.rs` | Marcadores en el mundo, cartel de interacción, caída de la antena. |
| `tacmap.rs` | Mapa táctico (Tab). |
| `perks.rs`, `shop.rs`, `subway.rs` | Pantallas de mejoras, tienda y subte (pausan la partida). |
| `daynight.rs` | Ciclo de día y noche, faroles y linterna. |

## Decisiones de rendimiento

Medido en el equipo de desarrollo (Intel i7-9750H + Radeon Pro 5300M):

- **Sombras**: eran lo más caro (~7 ms). Bevy las recalcula cada cuadro (la web las calculaba una vez);
  se acortaron por calidad (cascadas y distancia).
- **Resolución**: en Retina se dibujaba a 2880×1800; ahora la escena va a escala por calidad.
- **Entidades**: Bevy cobra CPU por entidad y por cámara. Se funden las armas (~100 piezas → ~10), los
  árboles y las zonas del mapa (88 m); los enemigos lejanos no se dibujan; solo el torso proyecta sombra.
- **Mapa de reflejos** de las armas: Bevy lo regeneraba cada cuadro; se sacó y se ajustaron los metales.
- La simulación compila optimizada también en desarrollo (`[profile.dev.package.arena_sim]`).

## Herramientas de desarrollo

| Variable / opción | Para qué |
|---|---|
| `--map NOMBRE` | ciudad, deposito, fundicion, cantera, azotea |
| `--at x,z,yaw,pitch` | Arrancar en un punto |
| `--weapon 1-4` | Arrancar con un arma |
| `--shot archivo.png` | Captura a los ~3 s y sale (si la ventana queda tapada, sale negra) |
| `ARENA_BENCH=1` | Mide ms por cuadro entre los 4 y los 10 s y sale |
| `ARENA_NO=gun,enemies,ai,shadows` | Apaga partes para medir su costo |
| `ARENA_Q=baja/media/alta` | Fuerza la calidad |
| `ARENA_HOOK=segundos` | Dispara el gancho en ese momento |
| `ARENA_DRIVE=1` | Arranca manejando el auto más cercano |
| `ARENA_TAB=1` | Mapa táctico abierto |

Perfiles: `cargo build --release` (para jugar, incremental) y `cargo build --profile dist` (distribución, LTO).
