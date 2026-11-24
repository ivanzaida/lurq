//! What mounted components keep to place one node where the chosen one of their items was laid out: the line of
//! tabs, the thumb of a segmented control. The node stands at the corner of what holds the items, moved by its
//! offset to where the item is, so that a recipe's transitions of its offset and its size slide it to the item chosen
//! next.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::core::{ElementRef, Signal, element_ref::RectObserver};

/// The refs of the items and of what holds them, where the node was last placed, and what hears the chosen item
/// laid out elsewhere.
#[derive(Clone)]
pub(crate) struct Slide {
  /// What holds the items: the node is placed in its coordinates.
  pub(crate) holder: ElementRef,
  /// A ref for each item, by its place, kept from one render to the next: where each was last laid out.
  items: Arc<Mutex<Vec<ElementRef>>>,
  /// Where the node was last placed.
  at: Arc<Mutex<Option<[f32; 4]>>>,
  /// Moved on where the chosen item is laid out elsewhere than the node was placed: what holds the items reads it,
  /// and renders again.
  pub(crate) moved: Signal<u64>,
  observer: Arc<Mutex<Option<RectObserver>>>,
}

impl Slide {
  pub(crate) fn new(moved: Signal<u64>) -> Self {
    Self {
      holder: ElementRef::new(),
      items: Arc::default(),
      at: Arc::default(),
      moved,
      observer: Arc::default(),
    }
  }

  /// The refs of `count` items, those kept and new ones after them.
  pub(crate) fn refs(&self, count: usize) -> Vec<ElementRef> {
    let mut items = self.items.lock();
    while items.len() < count {
      items.push(ElementRef::new());
    }
    items[..count].to_vec()
  }

  /// Where `item` was last laid out in `holder`: its start and top in it, its width and its height. `None` before it
  /// was laid out.
  pub(crate) fn place(item: &ElementRef, holder: &ElementRef) -> Option<[f32; 4]> {
    let (x, y, width, height) = item.rect();
    let (left, top, ..) = holder.rect();
    (width > 0.0 && height > 0.0).then_some([x - left, y - top, width, height])
  }

  /// Keeps `place` as where the node now stands, and hears `item` laid out in `holder` before the frame is painted:
  /// laid out elsewhere (the first time, or in a holder that changed), it moves [`Self::moved`] on, so that the frame
  /// shows the node where the item is.
  pub(crate) fn follow(&self, item: &ElementRef, holder: &ElementRef, place: Option<[f32; 4]>) {
    *self.at.lock() = place;
    let (at, moved, holder) = (self.at.clone(), self.moved.clone(), holder.clone());
    let observer = item.add_rect_observer_before_paint(move |rect| {
      let (x, y, ..) = holder.rect();
      let place = [rect.x - x, rect.y - y, rect.width, rect.height];
      if *at.lock() != Some(place) {
        moved.update(|moved| *moved += 1);
      }
    });
    *self.observer.lock() = Some(observer);
  }
}
