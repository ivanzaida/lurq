//! The checks of a video (`media/video.rs`) on macOS, where the system's player of a video is the main thread's:
//! the main thread runs its loop here, as an app's does, and the checks run on a thread beside it. Under the test
//! harness the main thread only waits, and no video would open.
//!
//! Every media is silenced before it plays. `cargo test -p lurq --features media --test media_video_apple`; a
//! name, or a part of one, runs the checks it is in.

#[cfg(target_os = "macos")]
#[path = "media/video.rs"]
#[allow(dead_code)]
mod video;

#[cfg(target_os = "macos")]
fn main() {
  #[link(name = "CoreFoundation", kind = "framework")]
  unsafe extern "C" {
    fn CFRunLoopRun();
  }
  type Check = (&'static str, fn());
  macro_rules! checks {
    ($($name:ident),* $(,)?) => { [$((stringify!($name), video::$name as fn())),*] };
  }
  let checks: Vec<Check> = checks![
    a_file_opens_with_its_size_its_length_and_its_first_frame,
    it_plays_by_its_clock_shows_frames_and_stops_where_it_is_paused,
    it_goes_where_it_is_sent_and_ends_at_its_end,
    bytes_play_as_a_file_does_and_what_is_no_video_says_why,
    a_clip_has_its_sound_and_what_is_only_heard_of_it_has_no_picture,
  ]
  .into_iter()
  .collect();
  // What cargo gives after `--`: a part of a name. What starts with `--` is the harness's, and is not one.
  let wanted: Vec<String> = std::env::args().skip(1).filter(|arg| !arg.starts_with("--")).collect();
  std::thread::spawn(move || {
    let mut failed = 0;
    let mut ran = 0;
    for (name, check) in checks {
      if !wanted.is_empty() && !wanted.iter().any(|part| name.contains(part.as_str())) {
        continue;
      }
      ran += 1;
      let passed = std::panic::catch_unwind(check).is_ok();
      println!("test {name} ... {}", if passed { "ok" } else { "FAILED" });
      failed += usize::from(!passed);
    }
    println!(
      "\ntest result: {}. {} passed; {failed} failed",
      if failed == 0 { "ok" } else { "FAILED" },
      ran - failed
    );
    std::process::exit(i32::from(failed != 0));
  });
  // SAFETY: the main thread's loop, which runs what is given to the main queue until the checks end the process.
  unsafe { CFRunLoopRun() };
}

#[cfg(not(target_os = "macos"))]
fn main() {}
