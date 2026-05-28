use std::{
  sync::{Arc, Mutex, mpsc},
  thread,
};

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
  pub fn new(size: usize) -> Self {
    assert!(size > 0, "thread pool size must be greater than 0");

    let (sender, receiver) = mpsc::channel::<Job>();
    let receiver = Arc::new(Mutex::new(receiver));

    let workers = (0..size).map(|id| Worker::new(id, Arc::clone(&receiver))).collect();

    Self {
      workers,
      sender: Some(sender),
    }
  }

  pub fn execute<F>(&self, job: F)
  where
    F: FnOnce() + Send + 'static,
  {
    let Some(sender) = &self.sender else {
      return;
    };

    sender.send(Box::new(job)).expect("failed to send job to worker thread");
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    drop(self.sender.take());

    for worker in &mut self.workers {
      if let Some(thread) = worker.thread.take() {
        thread.join().expect("worker thread panicked");
      }
    }
  }
}

struct Worker {
  #[allow(dead_code)]
  id: usize,
  thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
  fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
    let thread = thread::spawn(move || {
      loop {
        let message = receiver.lock().expect("receiver mutex poisoned").recv();

        match message {
          Ok(job) => job(),
          Err(_) => break,
        }
      }
    });

    Self {
      id,
      thread: Some(thread),
    }
  }
}
