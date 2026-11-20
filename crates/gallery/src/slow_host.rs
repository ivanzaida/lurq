//! A host on this machine that answers slowly: what the Picture page's "What loads first" and the Video Player
//! page's "From a slow host" load from, so that the order the app's loader reads in, and what a video asks of a
//! host, can be seen. It answers a picture a third of a second after it was asked for, with a small picture of one
//! color, and keeps which pictures it was asked for, in order. It serves a clip by the ranges it is asked for, so
//! many bytes a second to its client in all, a quarter of a second after each new connection, and keeps what it
//! was asked for of it.

use std::{
  io::{Read, Write},
  net::TcpListener,
  sync::{Arc, Mutex, OnceLock},
  time::{Duration, Instant},
};

/// How long the host takes over each picture.
const PAUSE: Duration = Duration::from_millis(350);
/// The side of a picture it serves, in pixels.
const SIDE: u32 = 24;
/// How many bytes of a clip the host gives its client a second, over all of the client's connections.
pub const CLIP_RATE: u64 = 192 * 1024;
/// How long the host takes over a new connection before it answers a request for a clip, as a handshake with
/// a host far away does.
pub const CLIP_CONNECTION: Duration = Duration::from_millis(250);
/// The clip it serves.
const CLIP: &[u8] = include_bytes!("../assets/media/sample.mp4");

pub struct SlowHost {
  address: String,
  kept: Arc<Kept>,
}

/// What the host was asked for.
#[derive(Default)]
struct Kept {
  /// The pictures, each its run and its number, in the order the requests came.
  pictures: Mutex<Vec<(u64, usize)>>,
  /// What was asked of the clips, each its run and what: `connection`, `HEAD`, or a range.
  clips: Mutex<Vec<(u64, String)>>,
  /// When the bytes of clips sent so far are all through: the next go after that, whichever connection sends
  /// them.
  through: Mutex<Option<Instant>>,
}

/// The gallery's slow host, started when first asked for. `None` where this machine gives it no port.
pub fn slow_host() -> Option<&'static SlowHost> {
  static HOST: OnceLock<Option<SlowHost>> = OnceLock::new();
  HOST.get_or_init(SlowHost::start).as_ref()
}

fn held<T>(kept: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
  kept.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl SlowHost {
  fn start() -> Option<Self> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let address = format!("http://{}", listener.local_addr().ok()?);
    let kept: Arc<Kept> = Arc::default();
    let told = kept.clone();
    std::thread::Builder::new()
      .name("gallery-slow-host".to_owned())
      .spawn(move || {
        for stream in listener.incoming().flatten() {
          let told = told.clone();
          let _ = std::thread::Builder::new()
            .name("gallery-slow-host-answer".to_owned())
            .spawn(move || answer(stream, &told));
        }
      })
      .ok()?;
    Some(Self { address, kept })
  }

  /// Where the picture `index` of the run `run` is: another run is other addresses, which nothing has loaded.
  pub fn picture(&self, run: u64, index: usize) -> String {
    format!("{}/{run}/{index}.bmp", self.address)
  }

  /// The pictures of the run `run` the host was asked for, in the order the requests came.
  pub fn asked(&self, run: u64) -> Vec<usize> {
    held(&self.kept.pictures)
      .iter()
      .filter(|(of, _)| *of == run)
      .map(|(_, index)| *index)
      .collect()
  }

  /// Where the clip of the run `run` is: another run is another address, which nothing has read.
  pub fn clip(&self, run: u64) -> String {
    format!("{}/clip/{run}.mp4", self.address)
  }

  /// What the host was asked of the clip of the run `run`, in order: `connection` for each new connection,
  /// `HEAD`, and the range of each request.
  pub fn clip_asked(&self, run: u64) -> Vec<String> {
    held(&self.kept.clips)
      .iter()
      .filter(|(of, _)| *of == run)
      .map(|(_, what)| what.clone())
      .collect()
  }
}

/// Answers the requests of one connection.
fn answer(mut stream: std::net::TcpStream, kept: &Kept) {
  let mut request = Vec::new();
  let mut chunk = [0_u8; 1024];
  let mut new = true;
  loop {
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
      match stream.read(&mut chunk) {
        Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
        _ => return,
      }
    }
    let said = String::from_utf8_lossy(&request).into_owned();
    request.clear();
    let path = said.split_whitespace().nth(1).unwrap_or_default();
    if let Some(run) = path
      .strip_prefix("/clip/")
      .and_then(|name| name.strip_suffix(".mp4"))
      .and_then(|run| run.parse::<u64>().ok())
    {
      if std::mem::take(&mut new) {
        held(&kept.clips).push((run, "connection".to_owned()));
        std::thread::sleep(CLIP_CONNECTION);
      }
      if !clip(&mut stream, kept, run, &said) {
        return;
      }
      continue;
    }
    new = false;
    // `GET /<run>/<index>.bmp`.
    let mut parts = path.trim_start_matches('/').trim_end_matches(".bmp").split('/');
    let named = parts
      .next()
      .and_then(|run| run.parse::<u64>().ok())
      .zip(parts.next().and_then(|index| index.parse::<usize>().ok()));
    let Some((run, index)) = named else {
      let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
      continue;
    };
    held(&kept.pictures).push((run, index));
    std::thread::sleep(PAUSE);
    let body = picture(index);
    let head = format!(
      "HTTP/1.1 200 OK\r\nContent-Type: image/bmp\r\nContent-Length: {}\r\n\r\n",
      body.len()
    );
    if stream.write_all(head.as_bytes()).is_err() || stream.write_all(&body).is_err() {
      return;
    }
  }
}

/// Answers one request for the clip of the run `run`: its head, or the range asked for, at the host's rate.
/// `false` once the connection is gone.
fn clip(stream: &mut std::net::TcpStream, kept: &Kept, run: u64, said: &str) -> bool {
  let len = CLIP.len();
  if said.starts_with("HEAD ") {
    held(&kept.clips).push((run, "HEAD".to_owned()));
    let head =
      format!("HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\nContent-Length: {len}\r\n\r\n");
    return stream.write_all(head.as_bytes()).is_ok();
  }
  let range = said
    .lines()
    .find_map(|line| {
      let (name, value) = line.split_once(':')?;
      name
        .eq_ignore_ascii_case("range")
        .then(|| value.trim().trim_start_matches("bytes=").to_owned())
    })
    .unwrap_or_else(|| "0-".to_owned());
  held(&kept.clips).push((run, range.clone()));
  let (from, to) = range.split_once('-').unwrap_or((&range, ""));
  let from: usize = from.trim().parse().unwrap_or(0);
  let to = to.trim().parse::<usize>().map_or(len - 1, |to| to.min(len - 1));
  if from >= len || from > to {
    let head =
      format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\n\r\n");
    return stream.write_all(head.as_bytes()).is_ok();
  }
  let head = format!(
    "HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {from}-{to}/{len}\r\nContent-Length: {}\r\n\r\n",
    to + 1 - from
  );
  if stream.write_all(head.as_bytes()).is_err() {
    return false;
  }
  for piece in CLIP[from..=to].chunks(2048) {
    let at = {
      let mut through = held(&kept.through);
      let now = Instant::now();
      let at = through.map_or(now, |through| through.max(now))
        + Duration::from_secs_f64(piece.len() as f64 / CLIP_RATE as f64);
      *through = Some(at);
      at
    };
    std::thread::sleep(at.saturating_duration_since(Instant::now()));
    if stream.write_all(piece).is_err() {
      return false;
    }
  }
  true
}

/// A picture of one color, the color its number gives: a BMP of 24 bits, which needs no encoder.
fn picture(index: usize) -> Vec<u8> {
  let turn = index as f32 * 0.61;
  let part = |shift: f32| (((turn + shift).sin() * 0.5 + 0.5) * 200.0 + 40.0) as u8;
  let (red, green, blue) = (part(0.0), part(2.1), part(4.2));
  let pixels = SIDE * SIDE * 3;
  let mut file = Vec::with_capacity(54 + pixels as usize);
  file.extend_from_slice(b"BM");
  file.extend_from_slice(&(54 + pixels).to_le_bytes());
  file.extend_from_slice(&[0; 4]);
  file.extend_from_slice(&54_u32.to_le_bytes());
  file.extend_from_slice(&40_u32.to_le_bytes());
  file.extend_from_slice(&SIDE.to_le_bytes());
  file.extend_from_slice(&SIDE.to_le_bytes());
  file.extend_from_slice(&1_u16.to_le_bytes());
  file.extend_from_slice(&24_u16.to_le_bytes());
  file.extend_from_slice(&[0; 24]);
  for _ in 0..SIDE * SIDE {
    file.extend_from_slice(&[blue, green, red]);
  }
  file
}
