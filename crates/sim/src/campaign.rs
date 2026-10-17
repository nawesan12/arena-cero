//! Campaña de operaciones (nuevo): ocho operaciones armadas a mano sobre el Distrito Sur,
//! encadenadas por la radio de Vera. Cada operación es una lista de pasos (`Step`) con un
//! objetivo (`Goal`), líneas de radio y acciones que se ejecutan al empezar (despertar un puesto,
//! mandar una escuadra, un camión que huye...). Al terminar cada paso queda un punto de control:
//! si caés, el paso vuelve a empezar desde ahí. Al final, rango S/A/B/C según tiempo, caídas y estilo.
//!
//! Todo es simulación: el cliente solo muestra `objective`, `marks`, la radio y el resultado.

use crate::city::town::{XS, ZS, block_center};
use crate::city::{CityData, Outpost, OutpostId, Role, UnitSpec};
use crate::enemies::{Alert, DroneKind, Kind, SoldierKind, new_drone, new_soldier};
use crate::game::{Game, GameEvent, Setup, Snd};
use crate::maps::Level;
use crate::objectives::{Actor, Prompt};
use crate::vehicle::{AiMode, CarAi, Vehicle};
use glam::{Vec2, Vec3};
use std::f32::consts::PI;

pub const VERA: &str = "VERA";
pub const ENEMY: &str = "RADIO ENEMIGA";
pub const N_OPS: usize = 10;

/// Nombre de cada operación (para el menú).
pub const OP_NAMES: [&str; N_OPS] = ["Llegada", "Antena", "Apagón", "Fuga de datos", "Arsenal", "La grúa", "Trenes", "Mando", "Nueve de Julio", "El Coloso"];

#[derive(Clone, Debug)]
pub enum Goal {
    /// Llegar a un punto (si está alto, hay que estar a esa altura).
    Reach { pos: Vec3, r: f32 },
    /// Llegar a donde quedó el último vehículo destruido.
    ReachWreck { r: f32 },
    /// Pararse a más de esta altura (subir a una azotea).
    Climb(f32),
    /// Mantener F en cada uno de estos puntos.
    Use { points: Vec<Vec3>, secs: f32, label: &'static str },
    /// Completar el objetivo de un puesto (la mecánica del mundo libre: F, carga, comandante).
    Outpost(OutpostId),
    /// Eliminar a los objetivos del paso (los que mandó `Squad { target: true }`).
    KillTargets,
    /// Marcar con T a los objetivos del paso.
    TagTargets,
    /// Quedarse en la zona hasta completar el tiempo, con oleadas de soldados.
    Hold { pos: Vec3, r: f32, secs: f32, every: f32, n: u32, op: Option<OutpostId> },
    /// Alejarse de un punto.
    Escape { from: Vec3, r: f32 },
    /// Subirse a un auto.
    Drive,
    /// Tomar el subte.
    Ride,
    /// Destruir los vehículos marcados antes de que terminen su ruta.
    StopCars,
}

#[derive(Clone, Debug)]
pub enum Action {
    /// Despierta un puesto (aparecen sus soldados y torretas).
    Wake(OutpostId),
    /// Escuadra de soldados. `op` les da navegación y puesto; `hunt` los manda directo a combate.
    Squad { at: Vec3, n: u32, role: Role, op: Option<OutpostId>, hunt: bool, target: bool, yaw: Option<f32> },
    /// Escuadra donde quedó el último vehículo destruido (con o sin el comandante).
    WreckSquad { n: u32, commander: bool },
    /// Drones cazadores.
    Hunters { at: Vec3, n: u32 },
    Alarm(OutpostId),
    /// Vehículos enemigos por una ruta: `flee` la recorren una vez y escapan; si no, persiguen.
    Cars { route: Vec<Vec2>, n: u32, hp: f32, crew: u32, flee: bool, target: bool, color: u8 },
    /// Auto libre para el jugador.
    ParkedCar { pos: Vec3, yaw: f32 },
    /// Apaga (destruye) las torretas de un puesto.
    TurretsOff(OutpostId),
    /// El comandante deja su puesto (se sube al blindado).
    RemoveCommander,
    /// Explosiones de decorado (tanques que vuelan).
    Booms(Vec<Vec3>),
    Blackout(bool),
    Refill,
    /// El Coloso: dron jefe enorme que caza al jugador (cuenta como objetivo del paso).
    Boss { at: Vec3, hp: f32 },
}

#[derive(Clone, Debug)]
pub struct Step {
    pub text: &'static str,
    pub goal: Goal,
    pub radio: Vec<(&'static str, &'static str)>,
    pub actions: Vec<Action>,
    /// Tiempo para cumplirlo (si se acaba, el paso vuelve a empezar).
    pub limit: Option<f32>,
    /// Punto de control desde este paso (si no, sigue el anterior).
    pub checkpoint: Option<Vec3>,
}

#[derive(Clone, Debug)]
pub struct OpDef {
    pub name: &'static str,
    /// Texto del briefing (antes de empezar).
    pub brief: &'static str,
    pub place: &'static str,
    /// Hora fija de la operación.
    pub hour: f32,
    /// Tiempo de referencia para el rango (s).
    pub par: f32,
    pub start: Vec3,
    pub steps: Vec<Step>,
    pub outro: Vec<(&'static str, &'static str)>,
}

impl OpDef {
    /// Lugares de la operación para el mapa del briefing: objetivos con posición y puestos.
    pub fn points(&self, c: &CityData) -> Vec<Vec3> {
        let mut v = Vec::new();
        for s in &self.steps {
            match &s.goal {
                Goal::Reach { pos, .. } | Goal::Hold { pos, .. } => v.push(*pos),
                Goal::Use { points, .. } => v.extend(points.iter().copied()),
                Goal::Outpost(id) => {
                    if let Some(o) = c.outposts.iter().find(|o| o.id == *id) {
                        v.push(Vec3::new(o.cx, 0.0, o.cz));
                    }
                }
                _ => {}
            }
        }
        v.dedup_by(|a, b| a.distance(*b) < 8.0);
        v
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    S,
    A,
    B,
    C,
}

impl Rank {
    pub fn letter(self) -> &'static str {
        match self {
            Rank::S => "S",
            Rank::A => "A",
            Rank::B => "B",
            Rank::C => "C",
        }
    }
    pub fn from_letter(s: &str) -> Option<Rank> {
        match s {
            "S" => Some(Rank::S),
            "A" => Some(Rank::A),
            "B" => Some(Rank::B),
            "C" => Some(Rank::C),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OpResult {
    pub op: usize,
    pub rank: Rank,
    /// 0..100
    pub rating: f32,
    pub time: f32,
    pub par: f32,
    pub kills: u32,
    pub deaths: u32,
    pub crits: u32,
    pub takedowns: u32,
    pub stealth: u32,
    pub score: u32,
    pub bonus: u32,
}

/// Puntaje del rango: 45 por tiempo, 35 por no caer y 20 por estilo (críticos, derribos y
/// bajas silenciosas en relación a las bajas).
pub fn rate(time: f32, par: f32, deaths: u32, kills: u32, crits: u32, takedowns: u32, stealth: u32) -> (f32, Rank) {
    let t = 45.0 * (par / time.max(1.0)).min(1.0);
    let d = 35.0 * (1.0 - 0.34 * deaths as f32).max(0.0);
    let style = (takedowns as f32 + stealth as f32 + crits as f32 / 3.0) / (kills as f32 * 0.6).max(3.0);
    let s = 20.0 * style.min(1.0);
    let r = t + d + s;
    let rank = if r >= 88.0 {
        Rank::S
    } else if r >= 72.0 {
        Rank::A
    } else if r >= 55.0 {
        Rank::B
    } else {
        Rank::C
    };
    (r, rank)
}

#[derive(Clone, Debug, Default)]
struct Base {
    score: u32,
    kills: u32,
    crits: u32,
    takedowns: u32,
    stealth: u32,
}

#[derive(Clone, Debug)]
pub struct Campaign {
    pub op: usize,
    pub def: OpDef,
    pub step: usize,
    /// Tiempo de la operación.
    pub t: f32,
    pub step_t: f32,
    /// Dónde reaparecés si caés (pies).
    pub checkpoint: Vec3,
    pub deaths: u32,
    /// Texto del objetivo actual (con progreso).
    pub objective: String,
    /// Marcadores del objetivo actual.
    pub marks: Vec<Vec3>,
    /// Tiempo que queda si el paso tiene límite.
    pub limit_left: Option<f32>,
    /// Progreso 0..1 del paso (defensa, puntos usados...).
    pub progress: Option<f32>,
    /// Operación terminada.
    pub result: Option<OpResult>,
    hold: f32,
    used: Vec<bool>,
    step_enemies: Vec<u32>,
    step_cars: Vec<u32>,
    targets: Vec<u32>,
    car_targets: Vec<u32>,
    wave_t: f32,
    last_wreck: Option<Vec3>,
    rode: bool,
    takedowns0: u32,
    outro_t: Option<f32>,
    base: Base,
}

fn op_of(c: &CityData, id: OutpostId) -> (usize, &Outpost) {
    c.outposts.iter().enumerate().find(|(_, o)| o.id == id).expect("puesto")
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Eje de la calle i (norte-sur) y de la j (este-oeste).
fn sx(i: usize) -> f32 {
    XS[i].c
}
fn sz(j: usize) -> f32 {
    ZS[j].c
}

/// Vuelta por los carriles de la derecha alrededor de las manzanas entre las calles i0..i1 y j0..j1.
fn loop_route(i0: usize, i1: usize, j0: usize, j1: usize) -> Vec<Vec2> {
    let l = 4.2;
    vec![v2(sx(i0) + l, sz(j0) + l), v2(sx(i1) - l, sz(j0) + l), v2(sx(i1) - l, sz(j1) - l), v2(sx(i0) + l, sz(j1) - l)]
}

fn v2(x: f32, z: f32) -> Vec2 {
    Vec2::new(x, z)
}

fn vera(t: &'static str) -> (&'static str, &'static str) {
    (VERA, t)
}

fn step(text: &'static str, goal: Goal) -> Step {
    Step { text, goal, radio: Vec::new(), actions: Vec::new(), limit: None, checkpoint: None }
}

impl Step {
    fn say(mut self, lines: &[(&'static str, &'static str)]) -> Self {
        self.radio.extend_from_slice(lines);
        self
    }
    fn act(mut self, a: Action) -> Self {
        self.actions.push(a);
        self
    }
    fn limit(mut self, s: f32) -> Self {
        self.limit = Some(s);
        self
    }
    fn cp(mut self, p: Vec3) -> Self {
        self.checkpoint = Some(p);
        self
    }
}

fn guard(at: Vec3, yaw: f32) -> Action {
    Action::Squad { at, n: 1, role: Role::Guard, op: None, hunt: false, target: true, yaw: Some(yaw) }
}

/// Las ocho operaciones. Las posiciones salen de los puestos del Distrito Sur.
pub fn ops(c: &CityData) -> Vec<OpDef> {
    use Action::*;
    use Goal::*;
    let o = |id| op_of(c, id).1;
    let mut v_ops = Vec::new();

    // 1 · Llegada: tutorial
    {
        v_ops.push(OpDef {
            name: OP_NAMES[0],
            brief: "Llegás al Distrito Sur. Antes de mandarte a pelear, Vera quiere ver cómo te movés: correr, el gancho, un derribo silencioso y manejar hasta el refugio.",
            place: "Barrio Oeste · 09:00",
            hour: 9.0,
            par: 170.0,
            start: v(sx(3), 0.16, sz(7) - 20.0),
            steps: vec![
                step("Corré hasta la esquina marcada", Reach { pos: v(sx(3), 0.0, sz(6)), r: 5.0 })
                    .say(&[vera("Acá Vera. Te escucho fuerte y claro. Bienvenido al Distrito Sur."), vera("Antes de meternos en serio, quiero ver cómo te movés. Corré hasta la esquina: Shift para correr.")]),
                step("Subí a una azotea con el gancho (E)", Climb(6.0)).say(&[vera("Bien. Ahora el gancho: apuntá a un borde alto y apretá E. Con Shift recogés cuerda, con Espacio te soltás con impulso.")]),
                step("Neutralizá al vigía (por la espalda, F)", KillTargets)
                    .act(guard(v(sx(3) + 3.0, 0.16, sz(6) - 25.0), PI))
                    .say(&[vera("Hay un vigía solo en la esquina de la avenida. Mira hacia el norte."), vera("Acercate por detrás, agachado con C, y apretá F. Si te ve, resolvelo como puedas.")]),
                step("Subí al auto (F)", Drive).act(ParkedCar { pos: v(sx(3) - 4.6, 0.0, sz(6) - 38.0), yaw: 0.0 }).say(&[vera("Limpio. Te dejé un auto en la avenida: subite con F.")]),
                step("Manejá hasta el refugio", Reach { pos: v(sx(2), 0.0, sz(7)), r: 8.0 }).say(&[vera("W acelera, S frena, Espacio es el freno de mano. Llevalo hasta el refugio, al oeste.")]),
                step("Entrá al refugio y reabastecete (F)", Reach { pos: c.safe_pos, r: 2.6 }).say(&[vera("Bajate con F. Adentro del refugio te reabastecés: integridad, munición y granadas.")]),
            ],
            outro: vec![vera("Perfecto. Ya sabés lo básico. Esta noche empezamos a desarmarlos, puesto por puesto.")],
        });
    }
    // 2 · Antena
    {
        let a = o(OutpostId::Antena);
        let (cx, cz) = (a.cx, a.cz);
        let out = v(cx + 22.0, 0.16, cz);
        v_ops.push(OpDef {
            name: OP_NAMES[1],
            brief: "La antena de comunicaciones les trae refuerzos cada vez que suena una alarma. Conseguí la tarjeta del oficial, apagá su torreta y volala.",
            place: "Antena de comunicaciones · 17:30",
            hour: 17.5,
            par: 240.0,
            start: v(sx(2) + 8.0, 0.16, cz),
            steps: vec![
                step("Eliminá al oficial con la tarjeta de acceso", KillTargets)
                    .act(Wake(OutpostId::Antena))
                    .act(Squad { at: v(cx + 8.0, 0.16, cz + 12.0), n: 1, role: Role::Patrol, op: Some(OutpostId::Antena), hunt: false, target: true, yaw: None })
                    .say(&[vera("Mientras tengan la antena, cada alarma les trae drones de refuerzo. Vamos a tirarla."), vera("El oficial de guardia lleva la tarjeta del panel. Te lo marqué. Sin ruido, si podés.")]),
                step("Usá la tarjeta en el panel", Use { points: vec![v(cx + 13.5, 1.0, cz + 4.5)], secs: 2.0, label: "USAR LA TARJETA" })
                    .cp(out)
                    .say(&[vera("Tenés la tarjeta. El panel está contra el muro este: con eso apago su torreta.")]),
                step("Plantá la carga en la antena", Outpost(OutpostId::Antena)).act(TurretsOff(OutpostId::Antena)).say(&[vera("Torreta fuera de línea. Ahora la carga, al pie de la antena. Mantené F.")]),
                step("¡Salí de ahí antes de que te encierren!", Escape { from: v(cx, 0.0, cz), r: 75.0 })
                    .limit(45.0)
                    .cp(out)
                    .act(Hunters { at: v(cx, 22.0, cz + 30.0), n: 2 })
                    .act(Squad { at: v(cx + 30.0, 0.16, cz + 20.0), n: 2, role: Role::Patrol, op: None, hunt: true, target: false, yaw: None })
                    .say(&[(ENEMY, "¡Perdimos la antena! ¡Todas las unidades al sector oeste!"), vera("Se cayó. Ahora salí de ahí: tenés unos segundos antes de que cierren la zona.")]),
            ],
            outro: vec![vera("Sin antena no pueden pedir refuerzos. Buen trabajo.")],
        });
    }
    // 3 · Apagón
    {
        let s = o(OutpostId::Sub);
        let (cx, cz) = (s.cx, s.cz);
        v_ops.push(OpDef {
            name: OP_NAMES[2],
            brief: "La subestación alimenta todas las torretas de la ciudad. Saboteá los transformadores y el generador, aguantá mientras Vera corta la red y escapá a oscuras.",
            place: "Subestación eléctrica · 22:00",
            hour: 22.0,
            par: 300.0,
            start: v(sx(7) + 8.0, 0.16, cz),
            steps: vec![
                step("Eliminá al vigía de la puerta norte", KillTargets)
                    .act(Wake(OutpostId::Sub))
                    .act(guard(v(cx, 0.16, cz - 19.0), PI))
                    .say(&[vera("La subestación alimenta las torretas de toda la ciudad. Esta noche se quedan a oscuras."), vera("Primero el vigía de la puerta norte. Mira para la calle.")]),
                step("Saboteá los 3 transformadores", Use { points: vec![v(cx - 8.0, 1.0, cz - 4.6), v(cx + 4.0, 1.0, cz - 4.6), v(cx - 8.0, 1.0, cz + 3.6)], secs: 2.0, label: "SABOTEAR EL TRANSFORMADOR" })
                    .cp(v(cx - 10.0, 0.16, cz - 19.0))
                    .say(&[vera("Tres transformadores, marcados. Mantené F en cada uno.")]),
                step("Apagá el generador", Outpost(OutpostId::Sub)).say(&[vera("Falta el generador, adentro del galpón.")]),
                step("Aguantá hasta que corte la red", Hold { pos: v(cx + 2.0, 0.0, cz + 1.0), r: 16.0, secs: 45.0, every: 10.0, n: 2, op: Some(OutpostId::Sub) })
                    .cp(v(cx + 10.0, 0.16, cz + 7.0))
                    .act(Blackout(true))
                    .act(Alarm(OutpostId::Sub))
                    .say(&[vera("Estoy cortando la red desde afuera, necesito 45 segundos. Quedate en la subestación."), (ENEMY, "¡Se cortó la luz en el sector este! ¡Vayan a la subestación!")]),
                step("Escapá a oscuras", Escape { from: v(cx, 0.0, cz), r: 90.0 })
                    .limit(60.0)
                    .act(Squad { at: v(cx - 34.0, 0.16, cz - 22.0), n: 2, role: Role::Patrol, op: None, hunt: true, target: false, yaw: None })
                    .say(&[vera("¡Listo, apagón total! Todas las torretas fuera. Salí de ahí; si no ves nada, L prende la linterna... pero te van a ver.")]),
            ],
            outro: vec![vera("La ciudad está a oscuras y sus torretas, muertas. Se van a acordar de esta noche.")],
        });
    }
    // 4 · Fuga de datos
    {
        let d = o(OutpostId::Datos);
        let (cx, cz) = (d.cx, d.cz);
        let obj = d.obj_pos.unwrap_or(v(cx, 1.0, cz));
        v_ops.push(OpDef {
            name: OP_NAMES[3],
            brief: "En el último piso de la torre de oficinas está el registro de todas sus unidades. Descargalo, defendé la subida y salí de ahí.",
            place: "Torre de oficinas · 11:00",
            hour: 11.0,
            par: 330.0,
            start: v(sx(7) + 8.0, 0.16, cz),
            steps: vec![
                step("Entrá a la torre de oficinas", Reach { pos: v(cx + 2.0, 0.0, cz), r: 4.0 })
                    .act(Wake(OutpostId::Datos))
                    .say(&[vera("En el último piso de esa torre tienen el registro de todas sus unidades. Lo quiero."), vera("Entrá por la puerta sur. De día te ven de lejos: usá la cobertura.")]),
                step("Subí al último piso", Reach { pos: v(cx + 2.0, 7.5, cz - 3.0), r: 9.0 }).say(&[vera("Escaleras arriba. Hay guardias en cada piso.")]),
                step("Descargá los datos (mantené F)", Outpost(OutpostId::Datos)).cp(v(obj.x - 2.0, obj.y - 0.84, obj.z + 2.0)).say(&[vera("Ahí está la terminal. Mantené F, yo hago el resto.")]),
                step("Defendé la subida de datos", Hold { pos: obj, r: 10.0, secs: 60.0, every: 12.0, n: 2, op: Some(OutpostId::Datos) })
                    .act(Alarm(OutpostId::Datos))
                    .say(&[vera("La copia sale lenta: 60 segundos. Detectaron la intrusión, vienen a cortarla."), (ENEMY, "¡Intrusión en la torre de oficinas! ¡Todos arriba!")]),
                step("Escapá de la torre", Escape { from: v(cx, 0.0, cz), r: 80.0 })
                    .limit(70.0)
                    .act(Hunters { at: v(cx, 25.0, cz + 30.0), n: 3 })
                    .say(&[vera("¡Lo tengo todo! Ahora vas a ver a cada uno de ellos marcado. Salí de ahí: el gancho es más rápido que la escalera.")]),
            ],
            outro: vec![vera("Con esos datos sabemos dónde está cada uno. Ya no se pueden esconder.")],
        });
    }
    // 5 · Arsenal
    {
        let r = o(OutpostId::Arsenal);
        let (cx, cz) = (r.cx, r.cz);
        let truck = vec![v2(250.0, sz(6) + 4.2), v2(sx(1) + 4.2, sz(6) + 4.2), v2(sx(1) + 4.2, sz(1) + 4.2), v2(250.0, sz(1) + 4.2)];
        v_ops.push(OpDef {
            name: OP_NAMES[4],
            brief: "El arsenal se abre con un código que viaja en un camión. Interceptalo antes de que termine su recorrido, forzá el arsenal y llevate el lanzagranadas.",
            place: "Arsenal · 14:00",
            hour: 14.0,
            par: 300.0,
            start: v(sx(5) + 2.0, 0.16, sz(6) + 18.0),
            steps: vec![
                step("Detené el camión con el código", StopCars)
                    .act(ParkedCar { pos: v(sx(5) - 4.6, 0.0, sz(6) + 14.0), yaw: PI })
                    .act(Cars { route: truck, n: 1, hp: 30.0, crew: 0, flee: true, target: true, color: 1 })
                    .say(&[vera("El arsenal se abre con un código que cambia cada día. Lo lleva ese camión."), vera("Va a cruzar la calle de adelante tuyo. Si se te escapa, perdimos el código. Tenés un auto al lado.")]),
                step("Tomá el código del camión", ReachWreck { r: 4.5 }).say(&[vera("¡Lo paraste! Revisá la cabina.")]),
                step("Forzá el arsenal con el código", Outpost(OutpostId::Arsenal))
                    .cp(v(sx(5), 0.16, sz(7)))
                    .act(Wake(OutpostId::Arsenal))
                    .say(&[vera("Código en mano. El arsenal está al oeste. Adentro hay un lanzagranadas con tu nombre.")]),
                step("Escapá de los refuerzos", Escape { from: v(cx, 0.0, cz), r: 80.0 })
                    .act(Cars { route: loop_route(3, 6, 6, 8), n: 2, hp: 18.0, crew: 2, flee: false, target: false, color: 3 })
                    .say(&[vera("Vienen dos patrullas. Probá el lanzagranadas contra los autos: tecla 4.")]),
            ],
            outro: vec![vera("Ahora tenemos con qué pelear de igual a igual.")],
        });
    }
    // 6 · La grúa
    {
        let ob = o(OutpostId::Obra);
        let (cx, cz) = (ob.cx, ob.cz);
        let cab = ob.obj_pos.unwrap_or(v(cx, 1.0, cz));
        v_ops.push(OpDef {
            name: OP_NAMES[5],
            brief: "Desde la cabina de la grúa se ve medio distrito. Subí, hackeala y marcá a los cinco cabecillas para eliminarlos uno por uno.",
            place: "Obra en construcción · 06:00",
            hour: 6.0,
            par: 300.0,
            start: v(sx(6), 0.16, sz(2) + 12.0),
            steps: vec![
                step("Subí a la cabina de la grúa (a 40 m)", Reach { pos: cab, r: 5.0 })
                    .act(Wake(OutpostId::Obra))
                    .say(&[vera("Amanece. Desde la cabina de la grúa se ve medio distrito."), vera("Engancháte de la pluma con E y subí. No mires abajo.")]),
                step("Hackeá la grúa", Outpost(OutpostId::Obra)).cp(v(cab.x - 1.0, cab.y - 1.0, cab.z)).say(&[vera("Conectate al panel de la cabina.")]),
                step("Marcá a los 5 cabecillas con T", TagTargets)
                    .act(Squad { at: v(sx(5), 0.16, cz + 10.0), n: 1, role: Role::Guard, op: None, hunt: false, target: true, yaw: None })
                    .act(Squad { at: v(sx(6), 0.16, cz - 10.0), n: 1, role: Role::Guard, op: None, hunt: false, target: true, yaw: None })
                    .act(Squad { at: v(cx, 0.16, sz(1)), n: 1, role: Role::Guard, op: None, hunt: false, target: true, yaw: None })
                    .act(Squad { at: v(cx + 10.0, 0.16, sz(2) - 6.0), n: 1, role: Role::Guard, op: None, hunt: false, target: true, yaw: None })
                    .act(Squad { at: v(sx(5) + 3.0, 0.16, sz(1) + 3.0), n: 1, role: Role::Guard, op: None, hunt: false, target: true, yaw: None })
                    .say(&[vera("Tengo cinco cabecillas en la zona, pero no sé cuál es cuál. Buscalos desde arriba y marcalos con T.")]),
                step("Eliminá a los 5 cabecillas", KillTargets).say(&[vera("Todos marcados. Desde ahí tenés buen ángulo: el riel atraviesa, tecla 3.")]),
            ],
            outro: vec![vera("Cinco menos. Sus francotiradores quedaron a la vista de todos.")],
        });
    }
    // 7 · Trenes
    {
        let tr = o(OutpostId::Trenes);
        let (cx, cz) = (tr.cx, tr.cz);
        let tanks = vec![v(cx + 12.6, 1.0, cz + 9.0), v(cx - 2.2, 1.0, cz + 10.0), v(cx - 4.0, 1.0, cz - 0.6)];
        let entrance = c.stations.first().map_or(v(0.0, 0.0, 0.0), |s| s.entrance);
        v_ops.push(OpDef {
            name: OP_NAMES[6],
            brief: "Sus drones vuelan con el combustible del patio de trenes. Volá los tanques y el depósito, y perdé a los perseguidores en el subte.",
            place: "Patio de trenes · 20:00",
            hour: 20.0,
            par: 330.0,
            start: v(sx(3), 0.16, sz(1) + 12.0),
            steps: vec![
                step("Plantá cargas en los 3 tanques", Use { points: tanks.clone(), secs: 1.5, label: "PLANTAR LA CARGA" })
                    .act(Wake(OutpostId::Trenes))
                    .say(&[vera("Sus drones vuelan con el combustible de ese depósito. Lo vamos a prender fuego."), vera("Tres cargas en los tanques y la principal en el depósito.")]),
                step("Plantá la carga principal", Outpost(OutpostId::Trenes)).say(&[vera("Ahora la principal. Cuando explote, corré.")]),
                step("¡Escapá de los perseguidores!", Escape { from: v(cx, 0.0, cz), r: 60.0 })
                    .limit(40.0)
                    .cp(v(sx(3), 0.16, sz(1)))
                    .act(Booms(tanks))
                    .act(Cars { route: loop_route(1, 4, 1, 3), n: 2, hp: 18.0, crew: 2, flee: false, target: false, color: 3 })
                    .say(&[vera("¡Fuego en todo el patio! Sin combustible, no hay más drones."), (ENEMY, "¡Atacaron el depósito! ¡Persíganlo!")]),
                step("Bajá al subte de la Plaza Mayor", Reach { pos: entrance, r: 4.0 }).say(&[vera("No los vas a perder en la calle. Bajá al subte de la Plaza Mayor: abajo no te ven.")]),
                step("Tomá el tren (F en el andén)", Ride).say(&[vera("Esperá el tren y subite con F cuando abra las puertas. Elegí cualquier estación.")]),
            ],
            outro: vec![vera("Los perdiste. Queda uno solo: el comandante.")],
        });
    }
    // 8 · Mando
    {
        let m = o(OutpostId::Mando);
        let (cx, cz) = (m.cx, m.cz);
        let park = block_center(6, 7);
        let plaza = v(park.x, 0.0, park.y);
        let armored = vec![v2(sx(8) - 4.2, cz), v2(sx(8) - 4.2, sz(6) - 4.2), v2(sx(2) + 4.2, sz(6) - 4.2), v2(sx(2) + 4.2, sz(2) + 4.2), v2(sx(8) - 4.2, sz(2) + 4.2), v2(sx(8) - 4.2, sz(4) - 4.2)];
        v_ops.push(OpDef {
            name: OP_NAMES[7],
            brief: "Sin antena, torretas ni drones, el comandante está solo con su guardia en la Casa de Gobierno. Esta vez no se escapa.",
            place: "Puesto de mando · 17:00",
            hour: 17.0,
            par: 420.0,
            start: v(sx(6), 0.16, sz(4) - 10.0),
            steps: vec![
                step("Asaltá el puesto de mando", Reach { pos: v(cx, 0.0, cz), r: 24.0 })
                    .act(Wake(OutpostId::Mando))
                    .say(&[vera("Es la última. El comandante está en su puesto, en el centro del distrito."), vera("Sin antena, sin torretas y sin drones, está solo con su guardia. Entrá.")]),
                step("¡Detené el blindado del comandante!", StopCars)
                    .cp(v(sx(7) + 3.5, 0.16, sz(5) - 6.0))
                    .act(RemoveCommander)
                    .act(Cars { route: armored, n: 1, hp: 45.0, crew: 0, flee: true, target: true, color: 2 })
                    .act(ParkedCar { pos: v(sx(8) + 4.6, 0.0, cz - 8.0), yaw: PI })
                    .say(&[(ENEMY, "¡Saquen al comandante! ¡Al blindado, ya!"), vera("¡Se escapa en un blindado! Hay un auto en la calle este. ¡Que no se te escape!")]),
                step("Eliminá al comandante", Outpost(OutpostId::Mando)).act(WreckSquad { n: 2, commander: true }).say(&[vera("¡Lo paraste! Salió del blindado con su escolta. Terminalo.")]),
                step("Llegá a la extracción (el Parque)", Reach { pos: plaza, r: 12.0 })
                    .act(Hunters { at: v(cx, 25.0, cz), n: 2 })
                    .act(Cars { route: loop_route(5, 7, 5, 7), n: 1, hp: 18.0, crew: 2, flee: false, target: false, color: 3 })
                    .say(&[vera("Se terminó. El helicóptero te espera en el Parque, al sur de la Plaza Mayor. Lo que queda de ellos te va a buscar.")]),
                step("Aguantá hasta que llegue el helicóptero", Hold { pos: plaza, r: 20.0, secs: 40.0, every: 9.0, n: 3, op: None })
                    .cp(plaza + Vec3::new(-9.0, 0.0, -6.0))
                    .say(&[vera("Cuarenta segundos. Aguantá en el parque."), (ENEMY, "¡Está en el parque! ¡Todos los que queden!")]),
            ],
            outro: vec![vera("Te veo. Subí... El comandante cayó."), vera("Esperá. Hay movimiento en el puerto. Esto no terminó.")],
        });
    }
    // 9 · Nueve de Julio: cargas en el Obelisco
    {
        let ob = v(sx(4), 0.0, sz(2));
        v_ops.push(OpDef {
            name: OP_NAMES[8],
            brief: "Lo que queda del ejército plantó cargas en el Obelisco para voltearlo antes de irse. Desactivalas a tiempo y defendé la avenida.",
            place: "Av. 9 de Julio y Corrientes · 19:30",
            hour: 19.5,
            par: 260.0,
            start: v(sx(3), 0.16, sz(2) + 32.0),
            steps: vec![
                step("Llegá al Obelisco", Reach { pos: ob, r: 22.0 })
                    .act(Squad { at: v(sx(4) + 10.0, 0.16, sz(2) + 16.0), n: 2, role: Role::Patrol, op: None, hunt: false, target: false, yaw: None })
                    .act(Squad { at: v(sx(4) - 10.0, 0.16, sz(2) - 16.0), n: 2, role: Role::Patrol, op: None, hunt: false, target: false, yaw: None })
                    .say(&[vera("Esto no es un cuartel: es el Obelisco. Pusieron cargas para tirarlo abajo."), vera("Hay guardias alrededor. Acercate por la 9 de Julio.")]),
                step("Desactivá las 3 cargas", Use { points: vec![v(sx(4) + 3.4, 1.0, sz(2)), v(sx(4), 1.0, sz(2) - 3.4), v(sx(4) + 12.0, 1.0, sz(2) + 8.0)], secs: 2.5, label: "DESACTIVAR LA CARGA" })
                    .limit(100.0)
                    .cp(v(sx(4) + 6.0, 0.16, sz(2) + 14.0))
                    .act(Hunters { at: v(sx(4), 24.0, sz(2) - 40.0), n: 2 })
                    .act(Squad { at: v(sx(4) + 14.0, 0.16, sz(1)), n: 3, role: Role::Patrol, op: None, hunt: true, target: false, yaw: None })
                    .say(&[vera("Tenés menos de dos minutos. Las tres cargas están marcadas: mantené F en cada una."), (ENEMY, "¡Está tocando las cargas! ¡Mátenlo!")]),
                step("Defendé la avenida", Hold { pos: ob, r: 22.0, secs: 45.0, every: 9.0, n: 3, op: None })
                    .say(&[vera("Desactivadas. Vienen por la avenida a terminar el trabajo. Aguantá 45 segundos.")]),
            ],
            outro: vec![vera("El Obelisco sigue en pie. Y la ciudad vio quién lo defendió.")],
        });
    }
    // 10 · El Coloso: jefe final en la Plaza Mayor
    {
        let pm = block_center(6, 4);
        let plaza = v(pm.x, 0.0, pm.y);
        v_ops.push(OpDef {
            name: OP_NAMES[9],
            brief: "Del puerto salió el Coloso, la máquina de guerra que guardaban para el final. Viene a la Plaza Mayor. Es ahora o nunca.",
            place: "Plaza Mayor · 18:30",
            hour: 18.5,
            par: 360.0,
            start: v(sx(5), 0.16, sz(4)),
            steps: vec![
                step("Llegá a la Plaza Mayor", Reach { pos: plaza, r: 18.0 })
                    .act(Squad { at: v(sx(6), 0.16, sz(4)), n: 2, role: Role::Patrol, op: None, hunt: true, target: false, yaw: None })
                    .act(Hunters { at: v(pm.x, 24.0, pm.y - 30.0), n: 2 })
                    .say(&[vera("Lo vemos en el radar: grande, lento y blindado. Va hacia la Plaza Mayor."), vera("Tenés el lanzagranadas y el riel. Usalos.")]),
                step("Derribá al Coloso", KillTargets)
                    .cp(v(pm.x - 14.0, 0.16, pm.y + 10.0))
                    .act(Boss { at: plaza + Vec3::Y * 14.0, hp: 220.0 })
                    .act(Squad { at: v(sx(7), 0.16, sz(5)), n: 3, role: Role::Patrol, op: None, hunt: true, target: false, yaw: None })
                    .say(&[(ENEMY, "Coloso en posición. Limpien la plaza."), vera("¡Ahí está! Apuntale al núcleo rojo: los críticos le hacen el doble.")]),
                step("Aguantá hasta que llegue la ayuda", Hold { pos: plaza, r: 20.0, secs: 35.0, every: 8.0, n: 3, op: None })
                    .say(&[vera("¡Cayó! Los que quedan vienen todos juntos. Treinta y cinco segundos más.")]),
            ],
            outro: vec![vera("Se terminó. De verdad, esta vez."), vera("El Distrito Sur es libre. Gracias por todo."), ("CRÉDITOS", "Arena Cero · hecho en Rust + Bevy · gracias por jugar")],
        });
    }
    v_ops
}

impl Game {
    /// Partida de campaña en la operación `op` (0..8). Las anteriores cuentan como hechas: la
    /// antena ya está caída, la red cortada, el combustible quemado, etc.
    pub fn new_campaign(level: &mut Level, op: usize) -> Game {
        let op = op.min(N_OPS - 1);
        let mut g = Game::new_with(level, 7 + op as u32, Setup::CAMPAIGN);
        let Some(c) = level.city.clone() else { return g };
        let defs = ops(&c);
        let def = defs[op].clone();
        // efectos de las operaciones anteriores
        let order = [None, Some(OutpostId::Antena), Some(OutpostId::Sub), Some(OutpostId::Datos), Some(OutpostId::Arsenal), Some(OutpostId::Obra), Some(OutpostId::Trenes), Some(OutpostId::Mando), None, None];
        if let Some(run) = g.city.as_mut() {
            for id in order[..op].iter().flatten() {
                let (oi, o) = op_of(&c, *id);
                if *id == OutpostId::Mando {
                    continue;
                }
                run.ops[oi].done = true;
                run.done.push(*id);
                match id {
                    OutpostId::Antena => {
                        run.antenna = false;
                        run.obj.antenna_fall = Some(10.0);
                        run.obj.fell = true;
                        for col in &o.antenna_cols {
                            level.world.remove_collider(*col);
                        }
                    }
                    OutpostId::Sub => run.powered = false,
                    OutpostId::Datos => run.intel = true,
                    OutpostId::Arsenal => run.armory = true,
                    OutpostId::Trenes => run.drones = false,
                    _ => {}
                }
            }
            if run.armory {
                g.events.push(GameEvent::Unlock);
            }
        }
        g.director.hour = def.hour;
        g.director.frozen = true;
        g.banner(format!("OPERACIÓN {} · {}", op + 1, def.name.to_uppercase()), def.place);
        let start = def.start;
        g.campaign = Some(Campaign {
            op,
            def,
            step: 0,
            t: 0.0,
            step_t: 0.0,
            checkpoint: start,
            deaths: 0,
            objective: String::new(),
            marks: Vec::new(),
            limit_left: None,
            progress: None,
            result: None,
            hold: 0.0,
            used: Vec::new(),
            step_enemies: Vec::new(),
            step_cars: Vec::new(),
            targets: Vec::new(),
            car_targets: Vec::new(),
            wave_t: 0.0,
            last_wreck: None,
            rode: false,
            takedowns0: 0,
            outro_t: None,
            base: Base::default(),
        });
        g.enter_step(level, 0, start, true);
        g
    }

    /// Dónde arranca el jugador y hacia dónde mira (hacia el primer objetivo).
    pub fn campaign_start(&self) -> Option<(Vec3, f32)> {
        let c = self.campaign.as_ref()?;
        let to = c.marks.first().copied().unwrap_or(c.def.start + Vec3::Z);
        let d = to - c.def.start;
        Some((c.def.start, (-d.x).atan2(-d.z)))
    }

    /// Señal del cliente: el jugador viajó en subte.
    pub fn campaign_rode(&mut self) {
        if let Some(c) = self.campaign.as_mut() {
            c.rode = true;
        }
    }

    fn enter_step(&mut self, level: &mut Level, i: usize, player: Vec3, radio: bool) {
        let Some(c) = self.campaign.as_mut() else { return };
        let Some(s) = c.def.steps.get(i).cloned() else { return };
        c.step = i;
        c.step_t = 0.0;
        c.hold = 0.0;
        c.wave_t = 0.0;
        c.rode = false;
        c.used = match &s.goal {
            Goal::Use { points, .. } => vec![false; points.len()],
            _ => Vec::new(),
        };
        c.limit_left = s.limit;
        // los objetivos siguen de un paso al otro, salvo que el paso traiga nuevos
        if s.actions.iter().any(|a| matches!(a, Action::Squad { target: true, .. } | Action::WreckSquad { commander: true, .. } | Action::Boss { .. })) {
            c.targets.clear();
        }
        if s.actions.iter().any(|a| matches!(a, Action::Cars { target: true, .. })) {
            c.car_targets.clear();
        }
        c.step_enemies.clear();
        c.step_cars.clear();
        if let Some(p) = s.checkpoint {
            c.checkpoint = p;
        }
        if i == 0 {
            c.base = Base { score: self.score, kills: self.kills, crits: self.crits, takedowns: self.takedowns, stealth: self.city.as_ref().map_or(0, |r| r.stealth_kills) };
        }
        c.takedowns0 = self.takedowns;
        if radio {
            for (who, text) in &s.radio {
                self.events.push(GameEvent::Radio { who: *who, text: (*text).to_string() });
            }
            if i > 0 {
                self.events.push(GameEvent::Sound(Snd::Objective));
            }
        }
        let cd = level.city.clone();
        for a in &s.actions {
            self.run_action(level, cd.as_ref(), a, player);
        }
    }

    fn run_action(&mut self, level: &mut Level, cd: Option<&CityData>, a: &Action, player: Vec3) {
        let Some(c) = cd else { return };
        let body = player + Vec3::Y * 1.2;
        match a {
            Action::Wake(id) => {
                let (oi, o) = op_of(c, *id);
                if self.city.as_ref().is_some_and(|r| r.ops[oi].asleep) {
                    let o = o.clone();
                    self.spawn_outpost(oi, &o, &mut level.world);
                }
            }
            Action::Squad { at, n, role, op, hunt, target, yaw } => {
                for k in 0..*n {
                    let p = *at + Vec3::new((k % 3) as f32 * 2.2 - 2.2 * ((n - 1).min(2) as f32) / 2.0, 0.0, (k / 3) as f32 * 2.2);
                    self.spawn_soldier(level, p, *role, op.map(|id| op_of(c, id).0), *hunt, *target, *yaw, body);
                }
            }
            Action::WreckSquad { n, commander } => {
                let at = self.campaign.as_ref().and_then(|c| c.last_wreck).unwrap_or(player);
                let mando = op_of(c, OutpostId::Mando).0;
                if *commander {
                    self.spawn_soldier(level, at + Vec3::new(2.0, 0.0, 0.0), Role::Commander, Some(mando), true, true, None, body);
                }
                for k in 0..*n {
                    self.spawn_soldier(level, at + Vec3::new(-2.0, 0.0, k as f32 * 2.0 - 1.0), Role::Guard, Some(mando), true, false, None, body);
                }
            }
            Action::Hunters { at, n } => {
                for k in 0..*n {
                    let id = self.next_id();
                    let mut e = new_drone(id, DroneKind::Normal, *at + Vec3::new(k as f32 * 3.0, 0.0, 0.0), 0, &mut self.rng);
                    e.hunting = true;
                    self.enemies.push(e);
                    if let Some(cp) = self.campaign.as_mut() {
                        cp.step_enemies.push(id);
                    }
                }
            }
            Action::Alarm(id) => {
                let oi = op_of(c, *id).0;
                self.alarm_public(oi, Some(body), level);
            }
            Action::Cars { route, n, hp, crew, flee, target, color } => {
                let dir = (route[1] - route[0]).normalize_or_zero();
                let yaw = dir.x.atan2(dir.y);
                for k in 0..*n {
                    let start = route[0] - dir * (k as f32 * 12.0);
                    let id = self.next_id();
                    let mut car = Vehicle::new(id, Vec3::new(start.x, 0.0, start.y), yaw, *color, &mut level.world);
                    car.hp = *hp;
                    let mut ai = CarAi::new(route.clone(), *crew, *flee, None);
                    if !*flee {
                        ai.mode = AiMode::Chase;
                    }
                    car.ai = Some(ai);
                    self.vehicles.push(car);
                    if let Some(cp) = self.campaign.as_mut() {
                        cp.step_cars.push(id);
                        if *target {
                            cp.car_targets.push(id);
                        }
                    }
                }
            }
            Action::ParkedCar { pos, yaw } => {
                let id = self.next_id();
                self.vehicles.push(Vehicle::new(id, *pos, *yaw, 4, &mut level.world));
                if let Some(cp) = self.campaign.as_mut() {
                    cp.step_cars.push(id);
                }
            }
            Action::TurretsOff(id) => {
                let oi = op_of(c, *id).0;
                for i in 0..self.enemies.len() {
                    if self.enemies[i].is_turret() && self.enemies[i].op == Some(oi) && self.enemies[i].alive() {
                        let hp = self.enemies[i].hp;
                        self.damage(i, hp + 1.0, None, false, body, level);
                    }
                }
            }
            Action::RemoveCommander => {
                for e in &mut self.enemies {
                    if e.kind == Kind::Soldier(SoldierKind::Commander) {
                        e.hp = 0.0;
                    }
                }
            }
            Action::Booms(points) => {
                for p in points {
                    self.events.push(GameEvent::Boom { pos: *p + Vec3::Y * 2.0, big: true });
                    self.events.push(GameEvent::Burst { pos: *p + Vec3::Y * 3.0, color: 0xff6a2e, n: 40, speed: 10.0, life: 1.4 });
                    self.events.push(GameEvent::Burst { pos: *p + Vec3::Y * 5.0, color: 0x2a2a2a, n: 24, speed: 5.0, life: 2.0 });
                }
                self.events.push(GameEvent::Sound(Snd::Boom));
                self.events.push(GameEvent::Shake(0.8));
            }
            Action::Blackout(b) => self.blackout = *b,
            Action::Boss { at, hp } => {
                let id = self.next_id();
                let mut e = new_drone(id, DroneKind::Boss, *at, 0, &mut self.rng);
                e.hunting = true;
                e.hp = *hp;
                e.max_hp = *hp;
                e.tagged = true;
                self.enemies.push(e);
                self.boss = Some(id);
                if let Some(cp) = self.campaign.as_mut() {
                    cp.step_enemies.push(id);
                    cp.targets.push(id);
                }
            }
            Action::Refill => self.events.push(GameEvent::Refill),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_soldier(&mut self, level: &Level, at: Vec3, role: Role, op: Option<usize>, hunt: bool, target: bool, yaw: Option<f32>, body: Vec3) {
        let y = if at.y > 1.0 { at.y } else { level.world.ground_at(at.x, at.z, at.y + 1.0) };
        let leash = if hunt { 60.0 } else if role == Role::Guard { 4.0 } else { 22.0 };
        let id = self.next_id();
        let mut e = new_soldier(id, op.unwrap_or(0), &UnitSpec { x: at.x, z: at.z, y, role, leash }, &mut self.rng);
        e.op = op;
        if let Some(yw) = yaw {
            e.yaw = yw;
            e.anchor_yaw = Some(yw);
        }
        if hunt {
            e.st = Alert::Combat;
            e.last_seen = body;
            e.see_t = 0.0;
            e.react = 0.8 + self.rng.f() * 0.5;
        }
        e.tagged = target;
        self.enemies.push(e);
        if let Some(cp) = self.campaign.as_mut() {
            cp.step_enemies.push(id);
            if target {
                cp.targets.push(id);
            }
        }
    }

    /// Un paso de la campaña. Llamar después de `step_objectives`.
    pub fn step_campaign(&mut self, level: &mut Level, a: &Actor, dt: f32) {
        let Some(cp) = self.campaign.as_mut() else { return };
        if cp.result.is_some() {
            return;
        }
        if let Some(t) = cp.outro_t.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.finish_op();
            }
            return;
        }
        cp.t += dt;
        cp.step_t += dt;
        let Some(s) = cp.def.steps.get(cp.step).cloned() else { return };
        // vehículos destruidos o que escaparon
        let mut escaped = false;
        for id in cp.step_cars.clone() {
            if let Some(car) = self.vehicles.iter().find(|v| v.id == id) {
                if car.wrecked && car.pos.y > -10.0 && cp.car_targets.contains(&id) {
                    cp.last_wreck = Some(car.pos);
                }
                if car.wrecked && car.pos.y < -10.0 && cp.car_targets.contains(&id) {
                    escaped = true;
                }
            }
        }
        let feet = Vec3::new(a.pos.x, a.feet, a.pos.z);
        // cooperativo: cualquiera de los jugadores cumple (llegar, subir, usar, escapar)
        let host_use = a.f_held && self.driving.is_none();
        let all: Vec<(Vec3, bool)> = std::iter::once((feet, host_use)).chain(self.pawns.iter().filter(|p| p.alive()).map(|p| (Vec3::new(p.state.pos.x, p.state.feet(), p.state.pos.z), p.input.use_held))).collect();
        let mut done = false;
        let mut fail: Option<&'static str> = None;
        let mut marks = Vec::new();
        let mut text = s.text.to_string();
        let mut progress = None;
        let near = |p: Vec3, r: f32| all.iter().any(|(f, _)| (p.x - f.x).hypot(p.z - f.z) < r && (p.y < 2.0 || f.y > p.y - 2.5));
        match &s.goal {
            Goal::Reach { pos, r } => {
                marks.push(*pos);
                done = near(*pos, *r);
            }
            Goal::ReachWreck { r } => {
                if let Some(p) = cp.last_wreck {
                    marks.push(p);
                    done = near(p, *r);
                }
            }
            Goal::Climb(h) => done = all.iter().any(|(f, _)| f.y > *h),
            Goal::Use { points, secs, label } => {
                let mut active = None;
                for (k, p) in points.iter().enumerate() {
                    if cp.used[k] {
                        continue;
                    }
                    marks.push(*p);
                    if all.iter().any(|(f, _)| (p.x - f.x).hypot(p.z - f.z) < 2.4 && (p.y - 1.0 - f.y).abs() < 1.4) {
                        active = Some(k);
                    }
                }
                let n_used = cp.used.iter().filter(|u| **u).count();
                if points.len() > 1 {
                    text = format!("{} ({}/{})", s.text, n_used, points.len());
                }
                progress = Some(n_used as f32 / points.len() as f32);
                if let Some(k) = active {
                    let pk = points[k];
                    let holding = all.iter().any(|(f, u)| *u && (pk.x - f.x).hypot(pk.z - f.z) < 2.4 && (pk.y - 1.0 - f.y).abs() < 1.4);
                    if holding {
                        cp.hold += dt;
                    } else {
                        cp.hold = 0.0;
                    }
                    let frac = (cp.hold / secs).min(1.0);
                    if let Some(run) = self.city.as_mut() {
                        run.obj.prompt = Some(Prompt { label: format!("MANTENÉ F · {label}"), frac: Some(frac) });
                    }
                    if cp.hold >= *secs {
                        cp.hold = 0.0;
                        cp.used[k] = true;
                        self.events.push(GameEvent::Sound(Snd::Plant));
                        self.events.push(GameEvent::Burst { pos: points[k], color: 0x5fd8ff, n: 20, speed: 5.0, life: 0.5 });
                        let cp = self.campaign.as_mut().unwrap();
                        done = cp.used.iter().all(|u| *u);
                    }
                } else {
                    cp.hold = 0.0;
                }
            }
            Goal::Outpost(id) => {
                if let Some(c) = level.city.as_ref() {
                    let (oi, o) = op_of(c, *id);
                    done = self.city.as_ref().is_some_and(|r| r.ops[oi].done);
                    let cmd = self.enemies.iter().find(|e| e.op == Some(oi) && e.alive() && e.kind == Kind::Soldier(SoldierKind::Commander));
                    match (cmd, o.obj_pos) {
                        (Some(e), _) => marks.push(e.pos + Vec3::Y * 1.4),
                        (None, Some(p)) => marks.push(p),
                        _ => {}
                    }
                    if let Some(ch) = self.city.as_ref().and_then(|r| r.obj.charge) {
                        if ch.op == oi {
                            text = format!("¡Alejate! Explota en {:.0}", ch.t.ceil());
                        }
                    }
                }
            }
            Goal::KillTargets | Goal::TagTargets => {
                let tag = matches!(s.goal, Goal::TagTargets);
                let mut ok = 0;
                for id in &cp.targets {
                    match self.enemies.iter().find(|e| e.id == *id && e.alive()) {
                        Some(e) if tag && e.tagged => ok += 1,
                        Some(e) => marks.push(e.pos + Vec3::Y * 2.3),
                        None => ok += 1,
                    }
                }
                let n = cp.targets.len();
                if n > 1 {
                    text = format!("{} ({ok}/{n})", s.text);
                }
                progress = Some(ok as f32 / n.max(1) as f32);
                done = ok >= n;
                if tag {
                    marks.clear();
                }
            }
            Goal::Hold { pos, r, secs, every, n, op } => {
                marks.push(*pos);
                let inside = (pos.x - feet.x).hypot(pos.z - feet.z) < *r;
                if inside {
                    cp.hold += dt;
                }
                cp.wave_t -= dt;
                let left = (secs - cp.hold).max(0.0);
                text = if inside { format!("{} · {:.0} s", s.text, left.ceil()) } else { format!("¡Volvé a la zona! · {:.0} s", left.ceil()) };
                progress = Some((cp.hold / secs).min(1.0));
                done = cp.hold >= *secs;
                if cp.wave_t <= 0.0 && !done {
                    cp.wave_t = *every;
                    let alive = cp.step_enemies.iter().filter(|id| self.enemies.iter().any(|e| e.id == **id && e.alive())).count();
                    let (pos, n, op) = (*pos, *n, *op);
                    if alive < 7 {
                        let oi = op.and_then(|id| level.city.as_ref().map(|c| op_of(c, id).0));
                        for _ in 0..n {
                            if let Some(p) = self.wave_spot(level, pos) {
                                self.spawn_soldier(level, p, Role::Patrol, oi, true, false, None, feet + Vec3::Y * 1.2);
                                if let Some(e) = self.enemies.last_mut() {
                                    e.anchor = pos;
                                }
                            }
                        }
                    }
                }
            }
            Goal::Escape { from, r } => {
                let d = all.iter().map(|(f, _)| (from.x - f.x).hypot(from.z - f.z)).fold(0.0f32, f32::max);
                text = format!("{} · {:.0} m", s.text, (r - d).max(0.0).ceil());
                progress = Some((d / r).min(1.0));
                done = d >= *r;
            }
            Goal::Drive => done = self.driving.is_some(),
            Goal::Ride => done = cp.rode,
            Goal::StopCars => {
                let mut stopped = 0;
                for id in &cp.car_targets {
                    if let Some(car) = self.vehicles.iter().find(|v| v.id == *id) {
                        if car.wrecked && car.pos.y > -10.0 {
                            stopped += 1;
                        } else if !car.wrecked {
                            marks.push(car.pos + Vec3::Y * 2.6);
                        }
                    }
                }
                done = stopped >= cp.car_targets.len() && !cp.car_targets.is_empty();
                if escaped {
                    fail = Some("Se escapó");
                }
            }
        }
        let cp = self.campaign.as_mut().unwrap();
        if let Some(l) = cp.limit_left.as_mut() {
            *l -= dt;
            if *l <= 0.0 && !done {
                fail = Some("Se acabó el tiempo");
            }
        }
        cp.objective = text;
        cp.marks = marks;
        cp.progress = progress;
        if let Some(why) = fail {
            self.banner("FALLASTE", format!("{why} · volvés al punto de control"));
            self.reset_step(level, true);
            return;
        }
        if done {
            let next = cp.step + 1;
            if next >= cp.def.steps.len() {
                cp.outro_t = Some(5.0);
                cp.objective = "Operación cumplida".into();
                cp.marks.clear();
                cp.limit_left = None;
                let outro = cp.def.outro.clone();
                for (who, t) in outro {
                    self.events.push(GameEvent::Radio { who, text: t.to_string() });
                }
                self.events.push(GameEvent::Sound(Snd::Objective));
                self.banner("OPERACIÓN CUMPLIDA", "");
            } else {
                if s.checkpoint.is_none() && cp.def.steps[next].checkpoint.is_none() && !matches!(s.goal, Goal::Escape { .. } | Goal::Drive | Goal::Climb(_)) && self.driving.is_none() {
                    // punto de control donde terminaste el paso (si estás en el piso)
                    if level.world.ground_at(feet.x, feet.z, feet.y + 0.3) > feet.y - 0.4 {
                        cp.checkpoint = feet;
                    }
                }
                self.events.push(GameEvent::Banner { title: "PUNTO DE CONTROL".into(), sub: String::new() });
                self.enter_step(level, next, feet, true);
            }
        }
    }

    /// Lugar libre para una oleada: en el piso, a 26-36 m de la zona.
    fn wave_spot(&mut self, level: &Level, center: Vec3) -> Option<Vec3> {
        for _ in 0..14 {
            let ang = self.rng.f() * std::f32::consts::TAU;
            let r = 26.0 + self.rng.f() * 10.0;
            let (x, z) = (center.x + ang.cos() * r, center.z + ang.sin() * r);
            let g = level.world.ground_at(x, z, 2.0);
            if g < 0.5 && level.world.box_at(Vec3::new(x, g + 1.0, z)).is_none() && x.abs() < level.world.lim() && z.abs() < level.world.lim() {
                return Some(Vec3::new(x, g, z));
            }
        }
        None
    }

    /// Caíste en la campaña: el paso vuelve a empezar desde el punto de control.
    pub fn campaign_down(&mut self, level: &mut Level) {
        self.reset_step(level, true);
    }

    fn reset_step(&mut self, level: &mut Level, death: bool) {
        let Some(cp) = self.campaign.as_mut() else { return };
        if death {
            cp.deaths += 1;
        }
        let (ids, cars, i, at) = (cp.step_enemies.clone(), cp.step_cars.clone(), cp.step, cp.checkpoint);
        for e in &mut self.enemies {
            if ids.contains(&e.id) {
                e.hp = 0.0;
            }
        }
        for v in &mut self.vehicles {
            if cars.contains(&v.id) {
                if let Some(col) = v.col.take() {
                    level.world.remove_collider(col);
                }
                v.wrecked = true;
                v.ai = None;
                v.pos.y = -60.0;
            }
        }
        self.on_player_down();
        if let Some(run) = self.city.as_mut() {
            run.obj.charge = None;
        }
        let feet_y = if at.y > 1.0 { at.y } else { level.world.ground_at(at.x, at.z, at.y + 1.0) };
        let pos = Vec3::new(at.x, feet_y, at.z);
        self.enter_step(level, i, pos, false);
        let to = self.campaign.as_ref().and_then(|c| c.def.steps.get(i)).map(|s| match &s.goal {
            Goal::Reach { pos, .. } | Goal::Hold { pos, .. } => *pos,
            Goal::Use { points, .. } => points[0],
            Goal::Escape { from, .. } => pos * 2.0 - *from,
            _ => pos + Vec3::Z,
        });
        let d = to.unwrap_or(pos + Vec3::Z) - pos;
        self.events.push(GameEvent::Respawn { pos, yaw: (-d.x).atan2(-d.z) });
    }

    fn finish_op(&mut self) {
        let stealth = self.city.as_ref().map_or(0, |r| r.stealth_kills);
        let Some(cp) = self.campaign.as_mut() else { return };
        let b = &cp.base;
        let kills = self.kills - b.kills;
        let crits = self.crits - b.crits;
        let takedowns = self.takedowns - b.takedowns;
        let stealth = stealth - b.stealth;
        let (rating, rank) = rate(cp.t, cp.def.par, cp.deaths, kills, crits, takedowns, stealth);
        let bonus = (rating * 40.0).round() as u32;
        self.score += bonus;
        let score = self.score - b.score;
        cp.result = Some(OpResult { op: cp.op, rank, rating, time: cp.t, par: cp.def.par, kills, deaths: cp.deaths, crits, takedowns, stealth, score, bonus });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemies::Target;
    use crate::maps::{MapId, build_level};

    fn target(p: Vec3) -> Target {
        Target { pos: p + Vec3::Y * 1.6, body: p + Vec3::Y * 1.2, vel: Vec3::ZERO, feet: p.y, crouch_t: 0.0, evasive: false, light: false }
    }

    #[test]
    fn rango_por_tiempo_caidas_y_estilo() {
        assert_eq!(rate(100.0, 200.0, 0, 10, 15, 3, 3).1, Rank::S);
        assert_eq!(rate(400.0, 200.0, 2, 10, 0, 0, 0).1, Rank::C);
        let (r1, _) = rate(200.0, 200.0, 0, 10, 0, 0, 0);
        let (r2, _) = rate(200.0, 200.0, 1, 10, 0, 0, 0);
        assert!(r1 > r2);
    }

    #[test]
    fn la_campana_no_trae_mundo_libre() {
        let mut l = build_level(MapId::Ciudad);
        let g = Game::new_campaign(&mut l, 1);
        assert!(g.loot.is_empty());
        assert!(!g.director.enabled);
        // solo despertó la antena (más el oficial)
        let run = g.city.as_ref().unwrap();
        let awake: Vec<_> = run.ops.iter().filter(|o| !o.asleep).collect();
        assert_eq!(awake.len(), 1);
        assert!(g.enemies.iter().all(|e| e.op.is_none() || !run.ops[e.op.unwrap()].asleep));
        assert_eq!(g.trains.len(), l.city.as_ref().unwrap().stations.len());
    }

    /// Un "bot" que va a cada objetivo, mantiene F, mata a los marcados y frena los autos:
    /// ninguna operación puede quedar trabada.
    #[test]
    fn cada_operacion_se_puede_terminar() {
        for op in 0..N_OPS {
            let mut l = build_level(MapId::Ciudad);
            let mut g = Game::new_campaign(&mut l, op);
            let mut p = g.campaign.as_ref().unwrap().def.start;
            let mut n = 0;
            while g.campaign.as_ref().unwrap().result.is_none() {
                n += 1;
                assert!(n < 60 * 900, "op {} trabada en el paso {} ({})", op + 1, g.campaign.as_ref().unwrap().step, g.campaign.as_ref().unwrap().objective);
                let cp = g.campaign.as_ref().unwrap();
                let s = cp.def.steps.get(cp.step).cloned();
                let mut f = false;
                if let Some(s) = s.filter(|_| cp.outro_t.is_none()) {
                    match &s.goal {
                        Goal::Reach { pos, .. } | Goal::Hold { pos, .. } => p = if pos.y < 2.0 { Vec3::new(pos.x, l.world.ground_at(pos.x, pos.z, 1.0), pos.z) } else { *pos - Vec3::Y * 0.5 },
                        Goal::ReachWreck { .. } => {
                            if let Some(w) = cp.marks.first() {
                                p = Vec3::new(w.x, 0.16, w.z);
                            }
                        }
                        Goal::Climb(h) => p.y = h + 1.0,
                        Goal::Use { .. } => {
                            if let Some(m) = cp.marks.first() {
                                p = *m - Vec3::Y;
                                f = true;
                            }
                        }
                        Goal::Outpost(id) => {
                            let (oi, o) = op_of(l.city.as_ref().unwrap(), *id);
                            let (ocx, ocz, obj_pos) = (o.cx, o.cz, o.obj_pos);
                            if let Some(op_p) = obj_pos {
                                p = op_p - Vec3::Y;
                                f = true;
                            }
                            // el bot es invencible y certero: mata al comandante
                            if let Some(i) = g.enemies.iter().position(|e| e.op == Some(oi) && e.kind == Kind::Soldier(SoldierKind::Commander) && e.alive()) {
                                let hp = g.enemies[i].hp;
                                g.damage(i, hp + 1.0, None, false, p, &mut l);
                            }
                            // si hay carga, alejarse
                            if g.city.as_ref().unwrap().obj.charge.is_some() {
                                p = Vec3::new(ocx + 30.0, 0.16, ocz);
                                f = false;
                            }
                        }
                        Goal::KillTargets | Goal::TagTargets => {
                            let ids = cp.targets.clone();
                            for i in 0..g.enemies.len() {
                                if ids.contains(&g.enemies[i].id) && g.enemies[i].alive() {
                                    if matches!(s.goal, Goal::TagTargets) {
                                        g.enemies[i].tagged = true;
                                    } else {
                                        let hp = g.enemies[i].hp;
                                        g.damage(i, hp + 1.0, None, false, p, &mut l);
                                    }
                                }
                            }
                        }
                        Goal::Escape { from, r } => {
                            let d = (p - *from).with_y(0.0).normalize_or(Vec3::X);
                            p = *from + d * (r + 5.0);
                            p.y = 0.16;
                        }
                        Goal::Drive => {
                            if let Some(i) = g.vehicles.iter().position(|v| !v.wrecked && v.ai.is_none()) {
                                g.driving = Some(i);
                            }
                        }
                        Goal::Ride => g.campaign_rode(),
                        Goal::StopCars => {
                            let ids = cp.car_targets.clone();
                            for i in 0..g.vehicles.len() {
                                if ids.contains(&g.vehicles[i].id) && !g.vehicles[i].wrecked {
                                    g.damage_vehicle(i, 999.0, &mut l, p + Vec3::Y * 500.0);
                                }
                            }
                        }
                    }
                }
                g.step(&mut l, &target(p), true, crate::TICK);
                let held = f && (n / 30) % 50 != 49;
                g.step_objectives(&mut l, &Actor { pos: p + Vec3::Y * 1.6, feet: p.y, f_held: held }, crate::TICK);
                g.step_campaign(&mut l, &Actor { pos: p + Vec3::Y * 1.6, feet: p.y, f_held: held }, crate::TICK);
                g.events.clear();
            }
            let r = g.campaign.as_ref().unwrap().result.clone().unwrap();
            eprintln!("op {} terminada en {} pasos de simulación, rango {:?}", op + 1, n, r.rank);
            assert_eq!(r.op, op);
        }
    }

    /// Inicios, puntos de control, paneles y escuadras: en lugares libres, no adentro de una pared.
    #[test]
    fn puntos_en_lugares_libres() {
        let l = build_level(MapId::Ciudad);
        let c = l.city.as_ref().unwrap();
        let mut bad = Vec::new();
        let free = |p: Vec3, what: String, bad: &mut Vec<String>| {
            let feet = if p.y > 2.0 { p.y - 1.0 } else { l.world.ground_at(p.x, p.z, p.y.max(0.0) + 1.0) };
            for h in [0.5, 1.2, 1.7] {
                if l.world.box_at(Vec3::new(p.x, feet + h, p.z)).is_some() {
                    bad.push(format!("{what} {p} (pies {feet})"));
                    return;
                }
            }
        };
        for (k, d) in ops(c).iter().enumerate() {
            free(d.start, format!("op {} inicio", k + 1), &mut bad);
            for (i, s) in d.steps.iter().enumerate() {
                if let Some(p) = s.checkpoint {
                    // los controles son pies: se prueba como si fuera un panel (pies + 1)
                    let p = if p.y > 1.0 { p + Vec3::Y } else { p };
                    free(p, format!("op {} paso {} control", k + 1, i + 1), &mut bad);
                }
                if let Goal::Use { points, .. } = &s.goal {
                    for p in points {
                        free(*p, format!("op {} paso {} panel", k + 1, i + 1), &mut bad);
                    }
                }
                for a in &s.actions {
                    match a {
                        Action::Squad { at, .. } => free(*at, format!("op {} paso {} escuadra", k + 1, i + 1), &mut bad),
                        Action::ParkedCar { pos, .. } => free(*pos, format!("op {} paso {} auto", k + 1, i + 1), &mut bad),
                        Action::Cars { route, .. } => {
                            for w in route.windows(2) {
                                let n = (w[0].distance(w[1]) / 2.0) as usize;
                                for j in 0..=n {
                                    let q = w[0].lerp(w[1], j as f32 / n.max(1) as f32);
                                    if l.world.box_at(Vec3::new(q.x, 1.0, q.y)).is_some() {
                                        bad.push(format!("op {} paso {} ruta bloqueada en {q}", k + 1, i + 1));
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    #[test]
    fn caer_vuelve_al_punto_de_control() {
        let mut l = build_level(MapId::Ciudad);
        let mut g = Game::new_campaign(&mut l, 1);
        let n0 = g.enemies.len();
        g.campaign_down(&mut l);
        let cp = g.campaign.as_ref().unwrap();
        assert_eq!(cp.deaths, 1);
        assert_eq!(cp.step, 0);
        assert!(g.events.iter().any(|e| matches!(e, GameEvent::Respawn { .. })));
        // el oficial vuelve a aparecer (el viejo se descarta en el próximo paso)
        g.step(&mut l, &target(cp.checkpoint), true, crate::TICK);
        assert_eq!(g.enemies.len(), n0);
    }
}
