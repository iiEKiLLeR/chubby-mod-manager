//! Co-op host system: one player hosts, friends join by IP. Everyone sees each other's
//! ping, whether their mods match the host's, and can download missing mods straight
//! from the host and start the game together.
//!
//! The original only synced trainer pokes for RE6 over port 9950 (config `network_port`,
//! default 0x26de set in function 0x1400074c0); this keeps the same default port.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub const PROTOCOL: u32 = 1;
pub const CHUNK: usize = 64 * 1024;
pub const MAX_FRAME: usize = 1 << 20;
/// Values from the original's Network_Start (function 0x140052b50).
pub const MAX_PLAYERS: usize = 128;
pub const CONNECT_ATTEMPTS: u32 = 12;
pub const CONNECT_INTERVAL_MS: u64 = 500;

fn epoch() -> Instant {
    static E: OnceLock<Instant> = OnceLock::new();
    *E.get_or_init(Instant::now)
}
pub fn now_ms() -> u64 {
    epoch().elapsed().as_millis() as u64
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModSig {
    pub key: String,
    pub source_name: String,
    pub name: String,
    pub crc: u32,
    pub size: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModSet {
    pub game: String,
    pub available: Vec<ModSig>,
    pub installed: Vec<String>, // keys, in install order
}

#[derive(Clone, Debug, Default)]
pub struct PlayerInfo {
    pub name: String,
    pub ping: u32,
    pub ready: bool,
    pub matches: bool,
    pub in_game: bool,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Hello { name: String, game: String, protocol: u32 },
    Roster(Vec<PlayerInfo>),
    Mods(ModSet),
    Ping(u64),
    Pong(u64),
    Chat(String, String),
    Want(String),
    Refuse(String),
    Launch,
    Ready(bool),
    InGame(bool),
    Welcome(u32),
    // File transfer frames are handled inside the connection threads.
    FileBegin { name: String, size: u64, crc: u32 },
    FileData(Vec<u8>),
    FileEnd,
}

// ---------- encoding ----------

struct W(Vec<u8>);
impl W {
    fn u8(&mut self, v: u8) { self.0.push(v); }
    fn u32(&mut self, v: u32) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn u64(&mut self, v: u64) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn s(&mut self, v: &str) { self.u32(v.len() as u32); self.0.extend_from_slice(v.as_bytes()); }
    fn b(&mut self, v: &[u8]) { self.u32(v.len() as u32); self.0.extend_from_slice(v); }
}
struct R<'a>(&'a [u8], usize);
impl R<'_> {
    fn take(&mut self, n: usize) -> io::Result<&[u8]> {
        if self.1 + n > self.0.len() { return Err(io::Error::new(io::ErrorKind::InvalidData, "short frame")); }
        let s = &self.0[self.1..self.1 + n];
        self.1 += n;
        Ok(s)
    }
    fn u8(&mut self) -> io::Result<u8> { Ok(self.take(1)?[0]) }
    fn u32(&mut self) -> io::Result<u32> { Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
    fn u64(&mut self) -> io::Result<u64> { Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap())) }
    fn s(&mut self) -> io::Result<String> { let n = self.u32()? as usize; Ok(String::from_utf8_lossy(self.take(n)?).to_string()) }
    fn b(&mut self) -> io::Result<Vec<u8>> { let n = self.u32()? as usize; Ok(self.take(n)?.to_vec()) }
}

pub fn encode(m: &Msg) -> Vec<u8> {
    let mut w = W(Vec::new());
    match m {
        Msg::Hello { name, game, protocol } => { w.u8(1); w.s(name); w.s(game); w.u32(*protocol); }
        Msg::Roster(list) => {
            w.u8(2); w.u32(list.len() as u32);
            for p in list { w.s(&p.name); w.u32(p.ping); w.u8(p.ready as u8); w.u8(p.matches as u8); w.u8(p.in_game as u8); }
        }
        Msg::Mods(set) => {
            w.u8(3); w.s(&set.game); w.u32(set.available.len() as u32);
            for a in &set.available { w.s(&a.key); w.s(&a.source_name); w.s(&a.name); w.u32(a.crc); w.u64(a.size); }
            w.u32(set.installed.len() as u32);
            for k in &set.installed { w.s(k); }
        }
        Msg::Ping(t) => { w.u8(4); w.u64(*t); }
        Msg::Pong(t) => { w.u8(5); w.u64(*t); }
        Msg::Chat(a, b) => { w.u8(6); w.s(a); w.s(b); }
        Msg::Want(n) => { w.u8(7); w.s(n); }
        Msg::Refuse(n) => { w.u8(8); w.s(n); }
        Msg::Launch => w.u8(9),
        Msg::Ready(r) => { w.u8(10); w.u8(*r as u8); }
        Msg::InGame(r) => { w.u8(11); w.u8(*r as u8); }
        Msg::Welcome(id) => { w.u8(12); w.u32(*id); }
        Msg::FileBegin { name, size, crc } => { w.u8(20); w.s(name); w.u64(*size); w.u32(*crc); }
        Msg::FileData(d) => { w.u8(21); w.b(d); }
        Msg::FileEnd => w.u8(22),
    }
    w.0
}

pub fn decode(b: &[u8]) -> io::Result<Msg> {
    let mut r = R(b, 0);
    Ok(match r.u8()? {
        1 => Msg::Hello { name: r.s()?, game: r.s()?, protocol: r.u32()? },
        2 => {
            let n = r.u32()?.min(64);
            let mut v = Vec::new();
            for _ in 0..n { v.push(PlayerInfo { name: r.s()?, ping: r.u32()?, ready: r.u8()? != 0, matches: r.u8()? != 0, in_game: r.u8()? != 0 }); }
            Msg::Roster(v)
        }
        3 => {
            let game = r.s()?;
            let n = r.u32()?.min(100_000);
            let mut available = Vec::new();
            for _ in 0..n { available.push(ModSig { key: r.s()?, source_name: r.s()?, name: r.s()?, crc: r.u32()?, size: r.u64()? }); }
            let n = r.u32()?.min(100_000);
            let mut installed = Vec::new();
            for _ in 0..n { installed.push(r.s()?); }
            Msg::Mods(ModSet { game, available, installed })
        }
        4 => Msg::Ping(r.u64()?),
        5 => Msg::Pong(r.u64()?),
        6 => Msg::Chat(r.s()?, r.s()?),
        7 => Msg::Want(r.s()?),
        8 => Msg::Refuse(r.s()?),
        9 => Msg::Launch,
        10 => Msg::Ready(r.u8()? != 0),
        11 => Msg::InGame(r.u8()? != 0),
        12 => Msg::Welcome(r.u32()?),
        20 => Msg::FileBegin { name: r.s()?, size: r.u64()?, crc: r.u32()? },
        21 => Msg::FileData(r.b()?),
        22 => Msg::FileEnd,
        k => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown message {}", k))),
    })
}

fn write_frame(s: &mut impl Write, m: &Msg) -> io::Result<()> {
    let body = encode(m);
    s.write_all(&(body.len() as u32).to_le_bytes())?;
    s.write_all(&body)?;
    s.flush()
}

fn read_frame(s: &mut impl Read) -> io::Result<Msg> {
    let mut len = [0u8; 4];
    s.read_exact(&mut len)?;
    let n = u32::from_le_bytes(len) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad frame size"));
    }
    let mut buf = vec![0u8; n];
    s.read_exact(&mut buf)?;
    decode(&buf)
}

// ---------- session ----------

#[derive(Debug)]
pub enum Event {
    Joined(u32, String, String),
    Left(u32, String),
    Msg(u32, Msg),
    Ping(u32, u32),
    FileProgress(String, u64, u64),
    FileDone(String, Result<(), String>),
    Error(String),
}

type Writer = Arc<Mutex<BufWriter<TcpStream>>>;

pub struct Session {
    pub hosting: bool,
    pub my_id: u32,
    pub port: u16,
    pub events: Receiver<Event>,
    tx: Sender<Event>,
    peers: Arc<Mutex<HashMap<u32, (Writer, TcpStream)>>>,
    pub incoming_dir: Arc<Mutex<PathBuf>>,
    stop: Arc<AtomicBool>,
    next_id: Arc<AtomicU32>,
}

impl Session {
    fn base(hosting: bool, port: u16, incoming: PathBuf) -> Session {
        let (tx, rx) = channel();
        Session {
            hosting,
            my_id: 0,
            port,
            events: rx,
            tx,
            peers: Arc::new(Mutex::new(HashMap::new())),
            incoming_dir: Arc::new(Mutex::new(incoming)),
            stop: Arc::new(AtomicBool::new(false)),
            next_id: Arc::new(AtomicU32::new(1)),
        }
    }

    /// Start hosting on `port` (0 picks a free port). Friends connect to this computer.
    pub fn host(port: u16, name: &str, game: &str, incoming: PathBuf) -> io::Result<Session> {
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        let real_port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let s = Session::base(true, real_port, incoming);
        let (peers, tx, stop, next) = (s.peers.clone(), s.tx.clone(), s.stop.clone(), s.next_id.clone());
        let dir = s.incoming_dir.clone();
        let (name, game) = (name.to_string(), game.to_string());
        thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                        let mut peek = [0u8; 4];
                        let is_http = matches!(stream.peek(&mut peek), Ok(4) if &peek == b"GET " || &peek == b"HEAD");
                        let _ = stream.set_read_timeout(None);
                        if is_http {
                            thread::spawn(move || serve_http_update(stream, real_port));
                            continue;
                        }
                        // The original's host allows 0x80 = 128 connections (function 0x140052b50).
                        if peers.lock().unwrap().len() >= MAX_PLAYERS {
                            let _ = stream.shutdown(Shutdown::Both);
                            continue;
                        }
                        let id = next.fetch_add(1, Ordering::SeqCst);
                        let hello = Msg::Hello { name: name.clone(), game: game.clone(), protocol: PROTOCOL };
                        spawn_conn(id, stream, peers.clone(), tx.clone(), dir.clone(), Some(hello), Some(Msg::Welcome(id)));
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(50)),
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                        thread::sleep(Duration::from_millis(200));
                    }
                }
            }
        });
        Ok(s)
    }

    /// Join a host at "ip" or "ip:port".
    pub fn join(addr: &str, default_port: u16, name: &str, game: &str, incoming: PathBuf) -> io::Result<Session> {
        let full = if addr.contains(':') { addr.to_string() } else { format!("{}:{}", addr.trim(), default_port) };
        let sa: SocketAddr = full
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "address not found"))?;
        // Same patience as the original's join (function 0x140052b50): 12 attempts, 500 ms apart.
        let mut last_err = io::Error::new(io::ErrorKind::TimedOut, "no answer");
        let mut stream = None;
        for _ in 0..CONNECT_ATTEMPTS {
            let t = Instant::now();
            match TcpStream::connect_timeout(&sa, Duration::from_millis(CONNECT_INTERVAL_MS)) {
                Ok(st) => {
                    stream = Some(st);
                    break;
                }
                Err(e) => last_err = e,
            }
            let spent = t.elapsed();
            if spent < Duration::from_millis(CONNECT_INTERVAL_MS) {
                thread::sleep(Duration::from_millis(CONNECT_INTERVAL_MS) - spent);
            }
        }
        let stream = stream.ok_or(last_err)?;
        let s = Session::base(false, sa.port(), incoming);
        let hello = Msg::Hello { name: name.to_string(), game: game.to_string(), protocol: PROTOCOL };
        spawn_conn(0, stream, s.peers.clone(), s.tx.clone(), s.incoming_dir.clone(), Some(hello), None);
        Ok(s)
    }

    pub fn peer_ids(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.peers.lock().unwrap().keys().copied().collect();
        v.sort();
        v
    }

    pub fn send(&self, id: u32, m: &Msg) {
        let w = self.peers.lock().unwrap().get(&id).map(|(w, _)| w.clone());
        if let Some(w) = w {
            let _ = write_frame(&mut *w.lock().unwrap(), m);
        }
    }

    pub fn broadcast(&self, m: &Msg) {
        for id in self.peer_ids() {
            self.send(id, m);
        }
    }

    /// Send a mod file to one peer on its own thread (pings keep flowing between chunks).
    pub fn send_file(&self, id: u32, path: PathBuf) {
        let w = self.peers.lock().unwrap().get(&id).map(|(w, _)| w.clone());
        let Some(w) = w else { return };
        let tx = self.tx.clone();
        thread::spawn(move || {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let res = (|| -> io::Result<()> {
                let size = fs::metadata(&path)?.len();
                let crc = crate::fsx::crc_file(&path)?;
                write_frame(&mut *w.lock().unwrap(), &Msg::FileBegin { name: name.clone(), size, crc })?;
                let mut f = BufReader::new(File::open(&path)?);
                let mut buf = vec![0u8; CHUNK];
                let mut sent = 0u64;
                loop {
                    let n = f.read(&mut buf)?;
                    if n == 0 { break; }
                    write_frame(&mut *w.lock().unwrap(), &Msg::FileData(buf[..n].to_vec()))?;
                    sent += n as u64;
                    let _ = tx.send(Event::FileProgress(format!("sending {}", name), sent, size));
                }
                write_frame(&mut *w.lock().unwrap(), &Msg::FileEnd)
            })();
            let _ = tx.send(Event::FileDone(format!("sent {}", name), res.map_err(|e| e.to_string())));
        });
    }

    pub fn close(&self) {
        self.stop.store(true, Ordering::SeqCst);
        for (_, (_, s)) in self.peers.lock().unwrap().drain() {
            let _ = s.shutdown(Shutdown::Both);
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}

fn spawn_conn(
    id: u32,
    stream: TcpStream,
    peers: Arc<Mutex<HashMap<u32, (Writer, TcpStream)>>>,
    tx: Sender<Event>,
    dir: Arc<Mutex<PathBuf>>,
    hello: Option<Msg>,
    welcome: Option<Msg>,
) {
    // Small messages go out at once: no Nagle delay on pings and roster updates.
    let _ = stream.set_nodelay(true);
    let Ok(wclone) = stream.try_clone() else { return };
    let Ok(kill) = stream.try_clone() else { return };
    let writer: Writer = Arc::new(Mutex::new(BufWriter::with_capacity(CHUNK + 64, wclone)));
    if let Some(w) = &welcome {
        let _ = write_frame(&mut *writer.lock().unwrap(), w);
    }
    if let Some(h) = &hello {
        let _ = write_frame(&mut *writer.lock().unwrap(), h);
    }
    peers.lock().unwrap().insert(id, (writer.clone(), kill));
    thread::spawn(move || {
        let mut reader = BufReader::with_capacity(CHUNK + 64, stream);
        let mut name = String::from("?");
        let mut incoming: Option<(String, PathBuf, BufWriter<File>, crc32fast::Hasher, u64, u64, u32)> = None;
        let reason = loop {
            let m = match read_frame(&mut reader) {
                Ok(m) => m,
                Err(e) => break e.to_string(),
            };
            match m {
                // Answered right here, not by the window, so the ping is the real network time
                // even while the window sleeps during a game.
                Msg::Ping(t) => {
                    let _ = write_frame(&mut *writer.lock().unwrap(), &Msg::Pong(t));
                }
                Msg::Pong(t) => {
                    let _ = tx.send(Event::Ping(id, now_ms().saturating_sub(t) as u32));
                }
                Msg::Hello { name: n, game, protocol } => {
                    if protocol != PROTOCOL {
                        let _ = tx.send(Event::Error(format!("{} uses a different version of Chubby Mod Manager", n)));
                    }
                    name = n.clone();
                    let _ = tx.send(Event::Joined(id, n, game));
                }
                Msg::FileBegin { name: fname, size, crc } => {
                    let safe = crate::mods::sanitize(&fname);
                    let d = dir.lock().unwrap().clone();
                    let _ = fs::create_dir_all(&d);
                    let part = d.join(format!("{}.part", safe));
                    match File::create(&part) {
                        Ok(f) => incoming = Some((safe, part, BufWriter::new(f), crc32fast::Hasher::new(), 0, size, crc)),
                        Err(e) => { let _ = tx.send(Event::FileDone(safe, Err(e.to_string()))); }
                    }
                }
                Msg::FileData(bytes) => {
                    if let Some((n, _, f, h, got, size, _)) = incoming.as_mut() {
                        h.update(&bytes);
                        let _ = f.write_all(&bytes);
                        *got += bytes.len() as u64;
                        let _ = tx.send(Event::FileProgress(n.clone(), *got, *size));
                    }
                }
                Msg::FileEnd => {
                    if let Some((n, part, mut f, h, got, size, crc)) = incoming.take() {
                        let _ = f.flush();
                        drop(f);
                        let ok = h.finalize() == crc && got == size;
                        let res = if ok {
                            let fin = part.with_file_name(&n);
                            crate::fsx::replace(&part, &fin).map_err(|e| e.to_string())
                        } else {
                            let _ = fs::remove_file(&part);
                            Err("file was damaged on the way (checksum mismatch)".into())
                        };
                        let _ = tx.send(Event::FileDone(n, res));
                    }
                }
                other => {
                    let _ = tx.send(Event::Msg(id, other));
                }
            }
        };
        if let Some((n, part, ..)) = incoming.take() {
            let _ = fs::remove_file(&part);
            let _ = tx.send(Event::FileDone(n, Err("connection lost".into())));
        }
        peers.lock().unwrap().remove(&id);
        let _ = tx.send(Event::Left(id, format!("{} ({})", name, reason)));
    });
}

/// This computer's address on the local network (no packets are sent).
pub fn local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".into())
}

/// What differs between my mods and the host's.
#[derive(Default, Clone, Debug)]
pub struct Diff {
    pub missing: Vec<ModSig>,      // host has it, I don't have the same file
    pub to_install: Vec<String>,   // host has it installed, I don't
    pub to_remove: Vec<String>,    // I have it installed, host doesn't
    pub order_differs: bool,
}
impl Diff {
    pub fn matches(&self) -> bool {
        self.missing.is_empty() && self.to_install.is_empty() && self.to_remove.is_empty() && !self.order_differs
    }
}

pub fn diff(host: &ModSet, mine: &ModSet) -> Diff {
    let mut d = Diff::default();
    for k in &host.installed {
        let hs = host.available.iter().find(|a| &a.key == k);
        let have = hs.map(|h| mine.available.iter().any(|m| m.key == h.key && m.crc == h.crc)).unwrap_or(false);
        if let Some(h) = hs {
            if !have && !d.missing.iter().any(|m| m.source_name == h.source_name) {
                d.missing.push(h.clone());
            }
        }
        if !mine.installed.contains(k) {
            d.to_install.push(k.clone());
        }
    }
    for k in &mine.installed {
        if !host.installed.contains(k) {
            d.to_remove.push(k.clone());
        }
    }
    if d.to_install.is_empty() && d.to_remove.is_empty() && host.installed != mine.installed {
        d.order_differs = true;
    }
    d
}

pub fn quality(ping: u32) -> (&'static str, [f32; 3]) {
    match ping {
        0..=60 => ("Great", [0.35, 0.85, 0.45]),
        61..=120 => ("OK", [0.95, 0.8, 0.3]),
        _ => ("Laggy", [0.95, 0.35, 0.3]),
    }
}

pub fn start_update_server() {
    let Ok(listener) = TcpListener::bind(("0.0.0.0", 9952)) else { return };
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || serve_http_update(stream, 9952));
        }
    });
}

pub fn serve_http_update(mut stream: TcpStream, default_port: u16) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));
    let mut req_buf = [0u8; 2048];
    let n = stream.read(&mut req_buf).unwrap_or(0);
    if n == 0 {
        return;
    }
    let req = String::from_utf8_lossy(&req_buf[..n]);
    let first_line = req.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");
    let host_hdr = req
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.trim().eq_ignore_ascii_case("host") {
                Some(v.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| format!("{}:{}", local_ip(), default_port));

    if path.ends_with(".exe") {
        if let Ok(exe) = std::env::current_exe() {
            if let Ok(bytes) = fs::read(&exe) {
                let hdr = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                );
                let _ = stream.write_all(hdr.as_bytes());
                let _ = stream.write_all(&bytes);
                let _ = stream.flush();
                return;
            }
        }
    }

    let dl_url = format!("http://{}/Chubby_Mod_Manager_Setup.exe", host_hdr);
    let body = crate::app::build_update_manifest_json(&dl_url);
    let hdr = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(hdr.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}
