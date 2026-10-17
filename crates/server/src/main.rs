//! Servidor dedicado de Arena Cero: corre la partida sin ventana y los jugadores se conectan.
//!
//! Uso: `arena-cero-servidor [--modo campana N | libre | oleadas N] [--puerto 47800] [--nombre "Mi servidor"]`
//! En la red local se anuncia solo; por internet hay que abrir el puerto UDP en el router.

use arena_net::host::NetHost;
use arena_net::{Mode, PORT};
use arena_sim::TICK;
use arena_sim::objectives::Actor;
use arena_sim::{Vec3, build_level};
use std::time::{Duration, Instant};

fn main() {
    let mut mode = Mode::Free;
    let mut port = PORT;
    let mut name = "Servidor Arena Cero".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--modo" => match args.next().as_deref() {
                Some("campana") | Some("campaña") => mode = Mode::Campaign(args.next().and_then(|v| v.parse::<u8>().ok()).unwrap_or(1).saturating_sub(1)),
                Some("oleadas") => mode = Mode::Waves(args.next().and_then(|v| v.parse::<u8>().ok()).unwrap_or(1).saturating_sub(1)),
                _ => mode = Mode::Free,
            },
            "--puerto" => port = args.next().and_then(|v| v.parse().ok()).unwrap_or(PORT),
            "--nombre" => name = args.next().unwrap_or(name),
            _ => eprintln!("opción desconocida: {a}"),
        }
    }
    let mut level = build_level(mode.map());
    let mut game = mode.new_game(&mut level);
    let mut host = match NetHost::new(port, mode, &name, false, true) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("no se pudo abrir el puerto {port}: {e}");
            std::process::exit(1);
        }
    };
    println!("Arena Cero · servidor dedicado «{name}» · {} · puerto UDP {}", mode.label(), host.port);
    // nadie juega del lado del servidor: un "jugador" lejos para lo que pide un actor
    let nobody = Actor { pos: Vec3::splat(1e5), feet: 1e5, f_held: false };
    let step = Duration::from_secs_f32(TICK);
    let mut next = Instant::now();
    let mut report = Instant::now();
    let mut done_t = 0.0f32;
    loop {
        host.receive(&mut game, &level);
        if !game.pawns.is_empty() {
            game.step_pawns(&mut level, TICK);
            let ts = game.all_targets(None);
            let full: Vec<bool> = game.pawns.iter().map(|p| p.hp >= p.max_hp).collect();
            game.step_multi(&mut level, &ts, &full, TICK);
            game.apply_pawn_hits(0);
            game.step_pawn_use(&mut level, TICK);
            game.step_objectives(&mut level, &nobody, TICK);
            game.step_campaign(&mut level, &nobody, TICK);
        }
        host.collect(&game);
        game.events.clear();
        host.send(&mut game, &level, None);
        // operación terminada: a los 12 s, la siguiente (o vuelve a la primera)
        if let Some(cp) = game.campaign.as_ref().filter(|c| c.result.is_some()) {
            done_t += TICK;
            if done_t > 12.0 {
                done_t = 0.0;
                let next_op = ((cp.op + 1) % arena_sim::campaign::N_OPS) as u8;
                let m = Mode::Campaign(next_op);
                level = build_level(m.map());
                let pawns: Vec<(u8, String)> = game.pawns.iter().map(|p| (p.id, p.name.clone())).collect();
                game = m.new_game(&mut level);
                for (id, n) in pawns {
                    game.add_pawn(&level, id, n);
                }
                host.new_game(m);
                println!("siguiente: {}", m.label());
            }
        }
        if report.elapsed() > Duration::from_secs(15) {
            report = Instant::now();
            let rtt: Vec<String> = host.rtt_ms().iter().map(|(i, r)| format!("J{i} {r:.0} ms")).collect();
            println!("{} jugadores · {} · {}", host.player_count(), mode.label(), rtt.join(" · "));
        }
        next += step;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else if now - next > Duration::from_millis(250) {
            next = now;
        }
    }
}
