//! Prueba de punta a punta por UDP en la misma máquina: un anfitrión, un cliente que se conecta,
//! saluda, recibe el mundo y camina; el anfitrión lo ve moverse.

use arena_net::client::NetClient;
use arena_net::host::{NetHost, host_snap};
use arena_net::{Mode, apply};
use arena_sim::pawn::PawnInput;
use arena_sim::{MapId, TICK, Vec3, build_level};
use std::time::{Duration, Instant};

#[test]
fn anfitrion_y_cliente_por_udp() {
    let port = 47917;
    let mode = Mode::Waves(0);
    let mut level = build_level(MapId::Deposito);
    let mut game = mode.new_game(&mut level);
    let mut host = NetHost::new(port, mode, "Anfitrión", true, false).expect("abrir el puerto");
    let mut client = NetClient::connect(format!("127.0.0.1:{port}").parse().unwrap(), "Amigo").expect("conectar");
    // el cliente arma su propia copia del mundo
    let mut level_c = build_level(MapId::Deposito);
    let mut game_c = mode.new_game(&mut level_c);
    let start = Instant::now();
    let mut start_pos: Option<Vec3> = None;
    let mut snaps = 0;
    while start.elapsed() < Duration::from_secs(6) {
        host.receive(&mut game, &level);
        game.step_pawns(&mut level, TICK);
        let host_t = arena_sim::enemies::Target { pos: Vec3::new(0.0, 1.76, 20.0), body: Vec3::new(0.0, 1.26, 20.0), vel: Vec3::ZERO, feet: 0.16, crouch_t: 0.0, evasive: false, light: false };
        let ts = game.all_targets(Some(host_t));
        let full = vec![true; ts.len()];
        game.step_multi(&mut level, &ts, &full, TICK);
        game.apply_pawn_hits(1);
        host.collect(&game);
        game.events.clear();
        host.send(&mut game, &level, Some(host_snap(host_t.pos, 0.0, 0.0, 0.0, 0.0, false, 0)));
        client.receive();
        if let Some(s) = client.snapshot.take() {
            snaps += 1;
            if let Some(you) = &s.you {
                start_pos.get_or_insert(you.pos);
            }
            apply(&mut game_c, s);
        }
        let walking = client.you.is_some() && start.elapsed() > Duration::from_secs(1);
        client.send_input(PawnInput { forward: if walking { 1.0 } else { 0.0 }, ..Default::default() });
        std::thread::sleep(Duration::from_secs_f32(TICK));
    }
    assert_eq!(client.you, Some(1), "el servidor lo aceptó como jugador 1");
    assert_eq!(client.players.len(), 2, "lista de jugadores: {:?}", client.players);
    assert!(snaps > 30, "llegaron fotos del mundo: {snaps}");
    let p = game.pawns.iter().find(|p| p.id == 1).expect("el remoto está en la partida");
    assert_eq!(p.name, "Amigo");
    let moved = p.state.pos.distance(start_pos.unwrap());
    assert!(moved > 5.0, "caminó {moved} m");
    // el cliente ve a los enemigos de la oleada
    assert_eq!(game_c.wave, game.wave);
}
