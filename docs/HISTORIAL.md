# Arena Cero — Historial del port

Del 16 al 17 de octubre de 2026. De una página web con Three.js a un juego nativo en Rust + Bevy,
y de ahí a un mundo abierto bastante más grande que el original.

## Punto de partida

`arena-cero.zip` traía la versión web (≈ 3.400 líneas de JavaScript, Three.js r128) partida en 30
módulos, un `CLAUDE.md` con la guía del port (que recomendaba Godot) y seis versiones anteriores del HTML.
La web tenía: ciudad procedural de 330 m con 7 puestos, IA táctica de soldados, torretas y drones,
4 armas con modelos procedurales, gancho, oleadas en 4 arenas, audio sintetizado con WebAudio y ranking
sobre el runtime de artifacts de claude.ai.

**Decisiones:** motor **Rust + Bevy** (en lugar de Godot); online **por etapas** (servicios →
cooperativo 2-4 → PvP); proyecto en `~/Projects/arena-cero`; repositorio público
[github.com/nawesan12/arena-cero](https://github.com/nawesan12/arena-cero).

## Etapas

1. **Base** (`776fe1f`): workspace con `arena_sim` sin Bevy (mundo de cajas, los 5 mapas con el mismo
   RNG que la web, movimiento completo) y cliente con texturas procedurales y mallas fundidas.
2. **Armas y calidad** (`0bdef72`, `b4bd78b`): arsenal en la simulación; calidad con tope de FPS
   (alta 120, nunca sin tope, 30 fuera de juego); los 4 modelos en primera persona portados pieza por
   pieza (perfiles extruidos con earcut); disparo, granadas, efectos y HUD.
3. **Audio** (`ebd29fc`): sintetizador propio con los efectos de la web y música en 4 capas por tensión.
4. **Enemigos** (`803739a`): drones, soldados, francotiradores, comandante, torretas, vigilancia y
   refuerzos; navegación A*; partida, oleadas, alarmas; HUD de combate.
5. **Interfaz y rendimiento** (`7283c26`): HUD rediseñado estilo shooter (brújula, vitales
   segmentados, chips de habilidad, pausa con controles); objetivos con F, derribo silencioso, marcado,
   reabastecimiento, antena que cae, victoria; marcadores y mapa táctico; mejoras; escala de resolución,
   sombras acortadas, piezas fundidas. CI en GitHub Actions (`d40915a`).
6. **Gancho visible y vehículos** (`369b7c5`, `50dc8d9`): lanzador, cable, garra y anillo; autos
   manejables con física arcade, atropellos y cámara en tercera persona.
7. **Mundo abierto** (`67bc8ac`): ciudad de 590 m con 4 barrios y 6 campamentos; patrullas en auto;
   misiones dinámicas (convoy, rehén, dron de carga, contraataque); créditos, botín, tienda y gadgets;
   día y noche con linterna.
8. **Ciudad 2.0 y gancho-péndulo**: colisiones con cajas giradas y huecos en el piso; Plaza Mayor,
   bulevar curvo, Diagonal Norte, rotondas, locales y puestos; subte con 3 estaciones; peatones y tránsito
   civil; gancho con física de péndulo, recoger y dar cuerda, envión y salida con impulso.

## Cambios respecto de la web

Fieles a la web: valores de movimiento y armas, IA y reglas de dificultad, diseño del Distrito Sur
(mismo RNG y orden de llamadas), las 4 arenas, los 7 objetivos, mejoras, efectos de sonido y música.

Corregido o cambiado a propósito:

- Las ruedas de los camiones mineros estaban giradas 90°.
- Los soldados no tenían navegación y se trababan: ahora usan A*.
- La viñeta oscura de apuntado se veía como "sombras negras alrededor del arma": se quitó.
- El riel está disponible desde el principio (pedido); el lanzagranadas sigue en el Arsenal.
- La dispersión dependía del aspecto de la pantalla; ahora usa un aspecto de referencia fijo.
- El gancho tiraba en línea recta; ahora es un péndulo.
- El ranking dependía de claude.ai y confiaba en el cliente: queda para la etapa online, validado.
- Los flanqueadores de las oleadas usan el rumbo de tu movimiento en vez del de tu cámara.

Nuevo (no estaba en la web): vehículos, ciudad ampliada, campamentos, patrullas en auto, misiones
dinámicas, economía, tienda, gadgets, día y noche, Ciudad 2.0, subte, peatones y tránsito.

## Campaña de operaciones (17/10/2026)

- Motor de campaña en `arena_sim::campaign` y las 8 operaciones; menú principal, radio, objetivo, resumen con rango.
- Pausa de verdad: con el mouse suelto la partida no avanza (antes los enemigos seguían disparando).
- Subte con trenes visibles: se sube solo con las puertas abiertas; viaje con fundido a negro.
- El piso se recorta justo alrededor de las bocas de subte (antes faltaba la baldosa de 8 m entera).
- Carteles en cola (no se pisan), faroles recalculados solo al moverse, apagón; `ARENA_NO` se lee una vez.

## Sensación arcade y ciudad menos repetida (17/10/2026)

- Hit-stop de 50 ms al matar (70 ms en racha), golpe de campo de visión y sacudida; los soldados se sacuden hacia atrás al recibir un golpe.
- Rachas en la ciudad: bajas dentro de 4 s suman (+50 por nivel), anunciador "DOBLE BAJA", "TRIPLE BAJA"... con sonido; "RACHA ×n" junto al puntaje.
- Lista de bajas y puntos a la derecha. Marcas de impacto en paredes y piso (pool de 64, normal calculada en la simulación).
- Disparos enemigos y drones con paneo estéreo (audio espacial de Bevy usado solo para panear).
- Fachadas: dos de ladrillo, dos de paneles de hormigón y cuatro de revoque (antes las 8 eran el mismo revoque).
- Decorado nuevo y solo visual (no consume el RNG de la ciudad): carteles publicitarios y neón en azoteas; pallets
  en el Puerto, bolardos y macetas en el Centro Financiero, cercos en el Barrio Jardín, tachos y cajones en El Cerro.

## Oleadas encadenadas, muertes y brillo (17/10/2026)

- Oleadas: cada 3 oleadas despejadas se pasa a la próxima arena (Depósito → Cantera → Fundición → Azotea) sin
  salir del juego, con puntaje, mejoras, armas y vida (`Game::next_arena`, `arena_cycle.rs`).
- Muertes según el golpe: de espaldas hacia donde empujó la bala (más fuerte si fue crítico), se desploma si fue
  un derribo, sale despedido y girando si fue una explosión (el cuerpo se arrastra sin atravesar paredes).
- Brillo (bloom) opcional con F5, apagado de entrada (pone HDR en las dos cámaras).
- Partículas en un pool fijo de 600 en vez de crear y borrar una entidad por partícula.

## Opciones, calles y radio (17/10/2026)

- Menú de **opciones**: sensibilidad del mouse, música, efectos, calidad, brillo y FPS (← → cambian, se guarda solo).
  La sensibilidad y el volumen de efectos ahora se guardan (`sens`, `efectos`).
- **Reiniciar la operación** desde el menú de pausa (Esc → Enter).
- El mapa táctico (Tab) muestra el objetivo de la campaña en dorado y los puestos dormidos en gris.
- **Nombres de calles**: al acercarte a una esquina aparecen los carteles de las dos calles.
- La radio de Vera tiene "voz": un pitido corto cada pocas letras mientras escribe.

## Lo que sigue

- Ajustar el brillo, la campaña y las rachas jugando.
- Ragdoll de verdad.
- Etapa online: servidor autoritativo (`arena_sim` headless), cuentas, ranking validado y cooperativo 2-4.
- Arte: modelos glTF para reemplazar los procedurales.
