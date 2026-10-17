# Arena Cero — Jugabilidad

Guía completa de lo que hay en el juego hoy. Para cómo está hecho, ver [ARQUITECTURA.md](ARQUITECTURA.md);
para la historia del port, [HISTORIAL.md](HISTORIAL.md).

## Modos

| Modo | Cómo se abre | Qué es |
|---|---|---|
| **Distrito Sur** (ciudad) | `arena-cero` | Mundo abierto de 590 m. Siete objetivos en el centro, cuatro barrios alrededor, misiones dinámicas, economía, vehículos, subte, día y noche. |
| **Oleadas** | `arena-cero --map deposito` (también `fundicion`, `cantera`, `azotea`) | Arenas cerradas con oleadas de drones, jefe cada 5 oleadas y mejoras entre oleadas. |

## Controles

| Tecla | Acción |
|---|---|
| W A S D | Moverse (manejando: acelerar, frenar/marcha atrás, doblar) |
| Shift | Correr · con el gancho: recoger cuerda |
| C | Agacharse · corriendo: deslizarse · con el gancho: dar cuerda |
| Espacio | Saltar (doble salto) · con el gancho: soltar con impulso · manejando: freno |
| Q | Dash |
| E / Ctrl | Gancho (otra vez lo suelta) |
| Clic izquierdo / derecho | Disparar / apuntar |
| 1-4 / rueda | Cambiar arma |
| R | Recargar |
| G | Granada |
| F | Usar: objetivos, derribo por la espalda, cajas, rehenes, reabastecerse, subir/bajar del auto, tomar el subte |
| T / clic de rueda | Marcar enemigo |
| Tab | Mapa táctico (ciudad) |
| B | Tienda (en un punto de reabastecimiento) |
| V / Z | Usar / cambiar gadget |
| L | Linterna |
| M | Música sí/no |
| F1 F2 F3 | Calidad baja / media / alta |
| F4 | Mostrar FPS |
| Esc | Pausa (pantalla con los controles) |

## Movimiento

Valores de la web, sin cambios: velocidad 7 m/s (10 corriendo), salto 8,2, gravedad 24, doble salto,
deslizamiento al agacharse corriendo, dash de 24 m/s durante 0,16 s cada 3 s.

### Gancho (rehecho)

En la web el gancho tiraba en línea recta. Ahora es un **péndulo**:

- Al engancharse, la cuerda queda un 10 % más corta que la distancia: el primer tirón te despega.
- La cuerda tiene **largo fijo**: te balanceás con la gravedad y conservás el impulso.
- **W/A/S/D** dan envión perpendicular a la cuerda (con W también un poco hacia arriba).
- **Shift** recoge cuerda (16 m/s) y te acerca; **C** da cuerda (hasta 55 m).
- **Espacio** suelta con toda la velocidad del balanceo, más un impulso hacia arriba y hacia adelante.
- Velocidad máxima 45 m/s; se suelta solo al llegar (< 1,8 m) o a los 8 s.
- Visual: lanzador en la muñeca izquierda, cable metálico, garra con aro cian y anillo cian en la mira
  cuando lo apuntado está a tiro (55 m). La vista se abre con la velocidad.

## Armas

| Arma | Cargador | Notas |
|---|---|---|
| Rifle de pulso | 24 | Automático, preciso |
| Escopeta | 6 | 9 perdigones |
| Riel de partículas | 4 | Atraviesa enemigos; mira de aumento. **Disponible desde el principio** (en la web se desbloqueaba) |
| Lanzagranadas | 3 | Explota al impactar. En la ciudad se desbloquea en el Arsenal o se compra en la tienda |

Críticos a la cabeza o al núcleo de los drones (×2, ×3 con la mejora Puntería fina). El escudo del
comandante rebota las balas. Los autos tapan las balas y se pueden destruir.

## Enemigos

- **Soldados** (fusil, francotirador con láser, comandante con escudo): patrullan, detectan de a poco
  (agachado ×0,5; corriendo ×1,45), buscan al oír ruido o encontrar cadáveres, en combate se cubren, se
  asoman, flanquean y tiran granadas. Reglas de la web: tardan 0,8-1,6 s en reaccionar cada vez que te
  vuelven a ver, como mucho dos tiradores por puesto. Ahora **navegan con A*** y no se traban.
- **Torretas** con láser (se apagan si saboteás la subestación).
- **Drones de vigilancia** con cono de luz por las avenidas: si te iluminan, alertan al puesto más cercano.
- **Refuerzos**: drones cazadores cada 28 s mientras haya alarma (sin antena no llegan).
- **Patrullas en auto** (nuevo): recorren calles, te persiguen si te ven y bajan dos soldados.
- En oleadas: drones, kamikazes, pesados y el Coloso.

De noche te ven hasta un 45 % menos lejos (salvo con la linterna prendida). El humo tapa la vista.

## Distrito Sur

### El mapa

- **Centro (Distrito Sur original):** los 7 puestos objetivo, autopista elevada, torres con pasarelas,
  obra con grúa, patio de trenes, refugio.
- **Puerto** (norte): industria, contenedores, tanques, segunda autopista, **Diagonal Norte** con locales.
- **Centro Financiero** (este): torres y el **bulevar curvo** con torres giradas siguiendo la curva.
- **Barrio Jardín** (sur): casas bajas con locales, patios, parques, puestos y la **Plaza Mayor**
  (fuente, glorieta, senderos en diagonal, bancos, feria de puestos).
- **El Cerro** (oeste): terrazas, mercados, departamentos con locales.
- **Rotondas** en cruces del anillo exterior.
- **6 campamentos** enemigos en los barrios: despejarlos los vuelve puntos de reabastecimiento.

### Objetivos (los 7 de la web)

| Puesto | Tarea (mantener F) | Efecto |
|---|---|---|
| Antena | Plantar carga | Sin refuerzos; la antena se cae |
| Subestación | Sabotear el generador | Torretas apagadas |
| Oficinas | Descargar datos (último piso) | Todos los enemigos marcados |
| Arsenal | Forzarlo | Desbloquea el lanzagranadas |
| Mando | Eliminar al comandante | — |
| Obra | Hackear la grúa (cabina a 40 m) | +1 intento; francotiradores y torretas marcados |
| Trenes | Plantar carga en el depósito | Sin drones |

Completar uno sin haber sido visto da +1000. Con los 7: victoria con bono por tiempo e intentos.
Tenés 3 intentos; al caer reaparecés en el punto de reabastecimiento más cercano.

### Misiones dinámicas (nuevo)

Aparecen cada 70-130 s cerca tuyo (máximo dos a la vez), marcadas en naranja en la brújula y en pantalla
con tiempo restante:

| Misión | Qué hacer | Recompensa |
|---|---|---|
| Convoy | Destruir los 3 autos antes de que crucen la ciudad | ¢ 700 |
| Rehén | Liberarlo manteniendo F (lo cuidan 3 soldados) | ¢ 600 |
| Dron de carga | Derribarlo (vuela con escolta) y juntar su caja | ¢ 400 + caja rica |
| Contraataque | Defender un puesto liberado de 5 soldados | ¢ 650 (si fallás, el puesto deja de reabastecer) |

### Economía (nuevo)

Créditos (¢) por bajas (25 soldado, 40 silenciosa, 300 comandante, 50 torreta), objetivos (400, 600 sin
ser visto), campamentos despejados (250), vehículos y misiones. Herir civiles cuesta 100.

**Cajas de botín** dentro de los edificios (80 por partida): créditos, munición, granadas o gadgets.

**Tienda** (B en un punto de reabastecimiento):

| Artículo | Precio | Efecto |
|---|---|---|
| Botiquín | 100 | Integridad al máximo |
| Granadas ×2 | 150 | Hasta el máximo |
| Humo ×2 | 180 | Gadget |
| Señuelo ×2 | 160 | Gadget |
| Dron explorador | 300 | Gadget |
| Munición pesada | 600 | +15 % de daño |
| Cargadores grandes | 450 | +25 % de capacidad |
| Placas de blindaje | 500 | +20 de integridad máxima |
| Lanzagranadas | 800 | Lo desbloquea |

**Gadgets** (V usa, Z cambia): **humo** (nube de 12 s que tapa la vista), **señuelo** (ruido cada 1,5 s
durante 10 s que atrae soldados), **dron explorador** (sube y marca a todos los enemigos en 50 m durante
12 s). Se arranca con un humo y un señuelo.

### Vehículos (nuevo)

Los autos de la calle se manejan (F para subir y bajar). Física arcade: hasta 108 km/h, marcha atrás,
dirección que se endurece con la velocidad, suben cordones y las rampas de las autopistas. Cámara en
tercera persona que se puede girar con el mouse. Adentro recibís la mitad del daño; los choques fuertes
lastiman. Podés atropellar soldados (el motor hace ruido y atrae enemigos). Se pueden robar autos del
tránsito. Los autos tienen vida: destruidos explotan y quedan como chatarra.

### Subte (nuevo)

Tres estaciones (Plaza Mayor, Refugio, Puerto Este) con boca en la calle, escalera, hall con andén y vías.
En el andén, F abre la lista y 1-3 te lleva a otra estación. Bajo tierra los enemigos no te ven.

### Vida en la ciudad (nuevo)

- 80 **peatones** caminan por las veredas; huyen de los tiros y las explosiones.
- 16 autos de **tránsito civil** circulan por las calles; frenan si estás adelante y escapan si les disparan.

### Día y noche (nuevo)

Un día completo dura 16 minutos (una hora de juego = 40 s; arranca a las 10:00). El sol cruza el cielo,
el atardecer se pone cálido, de noche hay luna, el cielo y la niebla se oscurecen, los faroles cercanos
se prenden y la linterna (L) alumbra. Reloj arriba a la izquierda.

## Mejoras

Se elige una de tres (1-3) después de cada objetivo y de cada oleada despejada: Blindaje, Munición
perforante, Manos rápidas, Cargador extendido, Servos, Nanorreparación, Absorción, Bandolera, Propulsores,
Impulso, Puntería fina (única).

## Interfaz

- **Arriba al centro:** brújula con objetivos, misiones, reabastecimiento y estaciones; alerta
  (oculto / buscando / detectado); barra del Coloso.
- **Arriba a la izquierda:** rombos de los 7 objetivos (u oleada), reloj, tiempo e intentos.
- **Arriba a la derecha:** puntaje, combo y créditos.
- **Abajo a la izquierda:** integridad segmentada, gadget activo y chips de dash, gancho y deslizamiento.
- **Abajo a la derecha:** munición (velocidad al manejar), barra de recarga, granadas.
- **Mira:** se abre con la dispersión; marca de impacto, de crítico y de baja; "+puntos" debajo.
- Marcadores en el mundo, números de daño, indicador de dirección del daño, viñeta roja al recibir daño.
- **Tab:** mapa táctico. **Esc:** pausa con controles.

## Calidad

| | Baja | Media | Alta |
|---|---|---|---|
| Tope de FPS | 40 | 60 | 120 |
| Resolución de la escena | 0,75× | 1× | 1,6× (hasta la nativa) |
| Sombras | 1024, 2 cascadas, 45 m | 2048, 3 cascadas, 80 m | 4096, 4 cascadas, 150 m |
| Distancia de dibujo | 200 m | 280 m | 360 m |

Siempre hay tope de FPS; fuera de juego (pausa o sin foco) baja a 30.
Las preferencias se guardan en `~/.arena-cero`.
