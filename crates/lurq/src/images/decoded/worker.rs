//! The threads that decode images, started with the first request and kept
//! for the process.

use std::{sync::mpsc, thread};

use parking_lot::Mutex;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub(super) struct Workers {
  sender: Mutex<mpsc::Sender<Job>>,
}

impl Workers {
  /// Starts `count` threads that run the submitted jobs in order of
  /// submission, one job per thread at a time.
  pub(super) fn new(count: usize) -> Self {
    let (sender, receiver) = mpsc::channel::<Job>();
    let receiver = std::sync::Arc::new(Mutex::new(receiver));
    for index in 0..count.max(1) {
      let receiver = std::sync::Arc::clone(&receiver);
      // A thread that cannot be spawned leaves the others to the jobs; with
      // none spawned, the receiver is gone, `submit` fails, and each job's
      // guard fails its image. The spawn error is logged.
      let spawned = thread::Builder::new()
        .name(format!("lurq-image-decode-{index}"))
        .spawn(move || {
          loop {
            let job = receiver.lock().recv();
            match job {
              // A job that panics (a decoder's bug) must not take the
              // worker with it: the other images still need one.
              Ok(job) => {
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).is_err() {
                  tracing::error!(target: "lurq::images", "an image decode job panicked; the worker goes on");
                }
              }
              Err(_) => break,
            }
          }
        });
      if let Err(error) = spawned {
        tracing::error!(target: "lurq::images", "image decode thread {index} did not start: {error}");
      }
    }
    Self {
      sender: Mutex::new(sender),
    }
  }

  /// `false` when no worker will ever take the job (the workers are gone);
  /// the job is dropped then.
  pub(super) fn submit(&self, job: impl FnOnce() + Send + 'static) -> bool {
    self.sender.lock().send(Box::new(job)).is_ok()
  }

  /// Workers that are gone: no job is ever taken.
  #[cfg(test)]
  pub(super) fn closed() -> Self {
    let (sender, _) = mpsc::channel::<Job>();
    Self {
      sender: Mutex::new(sender),
    }
  }
}
