//! Cliente: se conecta, saluda con su nombre, manda controles y recibe el mundo.

use crate::{CH_FAST, CH_RELIABLE, Mode, NetEvent, PROTOCOL_ID, Snapshot, ToClient, ToServer, connection_config, pack, unpack};
use arena_sim::pawn::PawnInput;
use renet::RenetClient;
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Instant, SystemTime};

pub struct NetClient {
    client: RenetClient,
    transport: NetcodeClientTransport,
    last: Instant,
    pub name: String,
    hello_sent: bool,
    /// Tu número de jugador (cuando el servidor te aceptó).
    pub you: Option<u8>,
    pub mode: Option<Mode>,
    pub players: Vec<(u8, String)>,
    /// Última foto que llegó (se consume al aplicarla).
    pub snapshot: Option<Snapshot>,
    pub events: Vec<NetEvent>,
    /// El servidor cambió de partida.
    pub new_game: Option<Mode>,
}

impl NetClient {
    pub fn connect(addr: SocketAddr, name: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        let client_id = now.as_millis() as u64 ^ u64::from(std::process::id());
        let auth = ClientAuthentication::Unsecure { protocol_id: PROTOCOL_ID, client_id, server_addr: addr, user_data: None };
        let transport = NetcodeClientTransport::new(now, auth, socket).map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(Self { client: RenetClient::new(connection_config()), transport, last: Instant::now(), name: name.to_owned(), hello_sent: false, you: None, mode: None, players: Vec::new(), snapshot: None, events: Vec::new(), new_game: None })
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn is_disconnected(&self) -> bool {
        self.client.is_disconnected()
    }

    pub fn rtt_ms(&self) -> f64 {
        self.client.rtt() * 1000.0
    }

    /// Recibe lo que llegó. Llamar una vez por cuadro.
    pub fn receive(&mut self) {
        let now = Instant::now();
        let dt = now - self.last;
        self.last = now;
        self.client.update(dt);
        let _ = self.transport.update(dt, &mut self.client);
        if self.client.is_connected() && !self.hello_sent {
            self.hello_sent = true;
            self.client.send_message(CH_RELIABLE, pack(&ToServer::Hello { name: self.name.clone(), version: PROTOCOL_ID }));
        }
        while let Some(b) = self.client.receive_message(CH_RELIABLE) {
            match unpack::<ToClient>(&b) {
                Some(ToClient::Welcome { you, mode, players }) => {
                    self.you = Some(you);
                    self.mode = Some(mode);
                    self.players = players;
                }
                Some(ToClient::Players(p)) => self.players = p,
                Some(ToClient::Events(v)) => self.events.extend(v),
                Some(ToClient::NewGame(m)) => {
                    self.mode = Some(m);
                    self.new_game = Some(m);
                }
                _ => {}
            }
        }
        while let Some(b) = self.client.receive_message(CH_FAST) {
            if let Some(ToClient::Snapshot(s)) = unpack::<ToClient>(&b) {
                // la más nueva gana (pueden llegar desordenadas)
                if self.snapshot.as_ref().is_none_or(|o| s.tick > o.tick) {
                    self.snapshot = Some(*s);
                }
            }
        }
    }

    /// Manda los controles de este paso.
    pub fn send_input(&mut self, input: PawnInput) {
        if self.client.is_connected() {
            self.client.send_message(CH_FAST, pack(&ToServer::Input(input)));
        }
        let _ = self.transport.send_packets(&mut self.client);
    }

    /// Pedir el viaje en subte a la estación `to`.
    pub fn ride(&mut self, to: u8) {
        if self.client.is_connected() {
            self.client.send_message(CH_RELIABLE, pack(&ToServer::Ride { to }));
        }
    }

    pub fn disconnect(&mut self) {
        self.transport.disconnect();
    }
}
