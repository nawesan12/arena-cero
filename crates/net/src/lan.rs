//! Partidas en la red local: el anfitrión se anuncia por broadcast UDP cada segundo y los clientes
//! escuchan y arman la lista.

use crate::{LAN_PORT, PROTOCOL_ID, pack, unpack};
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Announce {
    pub protocol: u64,
    pub name: String,
    pub port: u16,
    pub players: u8,
    pub mode: String,
}

/// Anuncio del anfitrión.
pub struct Beacon {
    socket: UdpSocket,
    name: String,
    port: u16,
    last: Instant,
}

impl Beacon {
    pub fn new(name: &str, port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;
        Ok(Self { socket, name: name.to_owned(), port, last: Instant::now() - Duration::from_secs(5) })
    }

    pub fn tick(&mut self, players: usize, mode: &str) {
        if self.last.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last = Instant::now();
        let a = Announce { protocol: PROTOCOL_ID, name: self.name.clone(), port: self.port, players: players as u8, mode: mode.to_owned() };
        let _ = self.socket.send_to(&pack(&a), SocketAddr::from(([255, 255, 255, 255], LAN_PORT)));
    }
}

/// Partida encontrada en la red local.
#[derive(Clone, Debug)]
pub struct Found {
    pub addr: SocketAddr,
    pub name: String,
    pub players: u8,
    pub mode: String,
    pub seen: Instant,
}

/// Escucha anuncios de la LAN.
pub struct Scanner {
    socket: Option<UdpSocket>,
    pub found: Vec<Found>,
}

impl Scanner {
    pub fn new() -> Self {
        let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], LAN_PORT))).ok();
        if let Some(s) = &socket {
            let _ = s.set_nonblocking(true);
        }
        Self { socket, found: Vec::new() }
    }

    pub fn poll(&mut self) {
        let Some(s) = &self.socket else { return };
        let mut buf = [0u8; 1024];
        while let Ok((n, from)) = s.recv_from(&mut buf) {
            let Some(a) = unpack::<Announce>(&buf[..n]) else { continue };
            if a.protocol != PROTOCOL_ID {
                continue;
            }
            let addr = SocketAddr::new(from.ip(), a.port);
            match self.found.iter_mut().find(|f| f.addr == addr) {
                Some(f) => {
                    f.players = a.players;
                    f.mode = a.mode;
                    f.seen = Instant::now();
                }
                None => self.found.push(Found { addr, name: a.name, players: a.players, mode: a.mode, seen: Instant::now() }),
            }
        }
        self.found.retain(|f| f.seen.elapsed() < Duration::from_secs(4));
    }
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}
