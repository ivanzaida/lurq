//! Medias a test drives: no thread, no decoder, nothing heard. What it opened, whether it waits, ended or failed
//! is the test's to say ([`MediaDriver`]); what it is told to do (play, pause, go to a time, a speed) it does in
//! the next turn of its thread's tree, as a media's thread does when it hears.

use std::{
  cell::RefCell,
  rc::Rc,
  sync::{Arc, Weak},
};

use super::{Command, Media, MediaError, MediaInfo, Shared};

thread_local! {
  /// The medias opened on this thread while a test drives them, in order: `None` while none does.
  static DRIVEN: RefCell<Option<Rc<RefCell<Vec<Arc<Shared>>>>>> = const { RefCell::new(None) };
  /// The medias made with [`Media::driven`] on this thread.
  static LOOSE: RefCell<Vec<Weak<Shared>>> = const { RefCell::new(Vec::new()) };
}

/// While it lives, every media opened on this thread ([`Media::open`], [`Media::sound`], a player's source) is
/// driven by the test that holds it, and opens nothing itself. For tests.
#[doc(hidden)]
pub struct DrivenMedias {
  opened: Rc<RefCell<Vec<Arc<Shared>>>>,
}

/// Has every media opened on this thread from now on be driven by the test, until what this returns is dropped.
#[doc(hidden)]
pub fn drive_media() -> DrivenMedias {
  let opened = Rc::new(RefCell::new(Vec::new()));
  DRIVEN.with(|driven| *driven.borrow_mut() = Some(opened.clone()));
  DrivenMedias { opened }
}

impl Drop for DrivenMedias {
  fn drop(&mut self) {
    DRIVEN.with(|driven| *driven.borrow_mut() = None);
  }
}

impl DrivenMedias {
  /// How many medias were opened since.
  pub fn count(&self) -> usize {
    self.opened.borrow().len()
  }

  /// The media opened `index`th.
  pub fn media(&self, index: usize) -> MediaDriver {
    MediaDriver(self.opened.borrow()[index].clone())
  }

  /// The media opened last.
  pub fn last(&self) -> MediaDriver {
    MediaDriver(self.opened.borrow().last().expect("no media was opened").clone())
  }
}

/// Whether medias are driven on this thread: a media opened now is then noted, and starts nothing.
pub(super) fn take(shared: &Arc<Shared>) -> bool {
  DRIVEN.with(|driven| match &*driven.borrow() {
    Some(opened) => {
      opened.borrow_mut().push(shared.clone());
      true
    }
    None => false,
  })
}

/// What a test says of a driven media.
#[doc(hidden)]
#[derive(Clone)]
pub struct MediaDriver(Arc<Shared>);

impl MediaDriver {
  /// It opened and is ready: its length, its picture's size and whether it has sound.
  pub fn open(&self, info: MediaInfo) {
    {
      let mut inner = self.0.inner();
      inner.duration = info.duration;
      inner.size = info.size;
      inner.sound = info.sound;
      inner.opened = true;
      inner.waiting = false;
      inner.hold();
    }
    self.0.wake();
  }

  /// It waits for what it plays, or has it again.
  pub fn wait(&self, waiting: bool) {
    {
      let mut inner = self.0.inner();
      inner.hold();
      inner.waiting = waiting;
    }
    self.0.wake();
  }

  /// It showed a frame, or its sound was heard: the first after it could move is its start
  /// ([`Media::on_started`](super::Media::on_started)).
  pub fn present(&self) {
    self.0.presented();
  }

  /// It played to its end: it ended, or where it is looped and plays, it goes on from its start.
  pub fn end(&self) {
    {
      let mut inner = self.0.inner();
      inner.hold();
      if inner.looped && inner.playing {
        inner.at = 0.0;
        inner.rounds += 1;
      } else {
        if let Some(duration) = inner.duration {
          inner.at = duration;
        }
        inner.ended = true;
        inner.playing = false;
      }
    }
    self.0.wake();
  }

  /// It cannot play, and why.
  pub fn fail(&self, why: MediaError) {
    self.0.fail(why);
  }

  /// What it was told and has not done yet, in order: play, pause, a time, a speed.
  pub fn pending(&self) -> usize {
    self.0.take_driven(false).len()
  }
}

impl Media {
  /// A media driven by the test that holds the driver, on no thread. For tests.
  #[doc(hidden)]
  pub fn driven() -> (Self, MediaDriver) {
    let shared = Shared::new();
    LOOSE.with(|loose| loose.borrow_mut().push(Arc::downgrade(&shared)));
    (Media::of(shared.clone()), MediaDriver(shared))
  }
}

/// Each driven media of this thread does what it was told: called by a tree in its turn, before it tells the news.
pub(super) fn answer() {
  let opened: Vec<Arc<Shared>> = DRIVEN.with(|driven| {
    driven
      .borrow()
      .as_ref()
      .map(|opened| opened.borrow().clone())
      .unwrap_or_default()
  });
  let loose: Vec<Arc<Shared>> = LOOSE.with(|loose| {
    let mut loose = loose.borrow_mut();
    loose.retain(|shared| shared.strong_count() > 0);
    loose.iter().filter_map(Weak::upgrade).collect()
  });
  for shared in opened.iter().chain(&loose) {
    answer_one(shared);
  }
}

fn answer_one(shared: &Shared) {
  // A media hears what it is told once it opened, as a decoder's thread does.
  if !shared.inner().opened && shared.inner().error.is_none() {
    return;
  }
  let commands = shared.take_driven(true);
  if commands.is_empty() {
    return;
  }
  {
    let mut inner = shared.inner();
    for command in commands {
      match command {
        Command::Play if inner.error.is_none() => {
          if inner.ended {
            inner.ended = false;
            inner.at = 0.0;
          }
          inner.hold();
          inner.playing = true;
        }
        Command::Pause => {
          inner.hold();
          inner.playing = false;
        }
        Command::Seek(to) => {
          inner.at = to;
          inner.ended = false;
        }
        Command::Rate(rate) => {
          inner.hold();
          inner.rate = rate;
        }
        _ => {}
      }
    }
  }
  shared.wake();
}
