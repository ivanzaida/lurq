use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
  pub const UNASSIGNED: Self = Self(0);

  pub fn value(self) -> u64 {
    self.0
  }

  pub fn is_assigned(self) -> bool {
    self.0 != 0
  }
}

#[derive(Clone)]
pub struct IdGenerator {
  pool: Arc<Mutex<IdPool>>,
}

struct IdPool {
  current: u64,
  freed: Vec<u64>,
}

impl IdGenerator {
  pub fn new() -> Self {
    Self {
      pool: Arc::new(Mutex::new(IdPool {
        current: 1,
        freed: Vec::new(),
      })),
    }
  }

  pub fn next(&self) -> NodeId {
    let mut pool = self.pool.lock().unwrap();
    let id = if let Some(id) = pool.freed.pop() {
      id
    } else {
      let id = pool.current;
      pool.current += 1;
      id
    };
    NodeId(id)
  }

  pub fn free(&self, id: NodeId) {
    if id.is_assigned() {
      self.pool.lock().unwrap().freed.push(id.0);
    }
  }
}
