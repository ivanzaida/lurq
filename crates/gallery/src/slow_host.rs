//! A host of pictures on this machine that answers slowly: what the Picture page's "What loads first" loads
//! from, so that the order the app's loader reads in can be seen. It answers every request a third of a second
//! after it came, with a small picture of one color, and keeps which pictures it was asked for, in order.

use std::{
  io::{Read, Write},
  net::TcpListener,
  sync::{Arc, Mutex, OnceLock},
  time::Duration,
};

/// How long the host takes over each picture.
const PAUSE: Duration = Duration::from_millis(350);
/// The side of a picture it serves, in pixels.
const SIDE: u32 = 24;

pub struct SlowHost {
  address: String,
  /// The pictures it was asked for, each its run and its number, in the order the requests came.
  asked: Arc<Mutex<Vec<(u64, usize)>>>,
}

/// The gallery's slow host, started when first asked for. `None` where this machine gives it no port.
pub fn slow_host() -> Option<&'static SlowHost> {
  static HOST: OnceLock<Option<SlowHost>> = OnceLock::new();
  HOST.get_or_init(SlowHost::start).as_ref()
}

impl SlowHost {
  fn start() -> Option<Self> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let address = format!("http://{}", listener.local_addr().ok()?);
    let asked: Arc<Mutex<Vec<(u64, usize)>>> = Arc::default();
    let kept = asked.clone();
    std::thread::Builder::new()
      .name("gallery-slow-host".to_owned())
      .spawn(move || {
        for stream in listener.incoming().flatten() {
          let kept = kept.clone();
          let _ = std::thread::Builder::new()
            .name("gallery-slow-host-answer".to_owned())
            .spawn(move || answer(stream, &kept));
        }
      })
      .ok()?;
    Some(Self { address, asked })
  }

  /// Where the picture `index` of the run `run` is: another run is other addresses, which nothing has loaded.
  pub fn picture(&self, run: u64, index: usize) -> String {
    format!("{}/{run}/{index}.bmp", self.address)
  }

  /// The pictures of the run `run` the host was asked for, in the order the requests came.
  pub fn asked(&self, run: u64) -> Vec<usize> {
    let asked = self.asked.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    asked
      .iter()
      .filter(|(of, _)| *of == run)
      .map(|(_, index)| *index)
      .collect()
  }
}

/// Answers the requests of one connection, each after the host's pause.
fn answer(mut stream: std::net::TcpStream, kept: &Mutex<Vec<(u64, usize)>>) {
  let mut request = Vec::new();
  let mut chunk = [0_u8; 1024];
  loop {
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
      match stream.read(&mut chunk) {
        Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
        _ => return,
      }
    }
    let said = String::from_utf8_lossy(&request).into_owned();
    request.clear();
    // `GET /<run>/<index>.bmp`.
    let path = said.split_whitespace().nth(1).unwrap_or_default();
    let mut parts = path.trim_start_matches('/').trim_end_matches(".bmp").split('/');
    let named = parts
      .next()
      .and_then(|run| run.parse::<u64>().ok())
      .zip(parts.next().and_then(|index| index.parse::<usize>().ok()));
    let Some((run, index)) = named else {
      let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
      continue;
    };
    kept
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .push((run, index));
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
