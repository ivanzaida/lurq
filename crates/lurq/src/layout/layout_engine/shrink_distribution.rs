//! Sharing a single-line Row/Column's overflow among its shrinking children.
//!
//! Children give way by order, lowest first: the overflow reaches the next
//! order only once every child of the current one is at its floor or gone.
//! Within one order, while its shrinking children cannot absorb the overflow
//! above their floors, children give way whole, in a fixed sequence, each
//! kind last first: those that drop (`ShrinkLimit::Drop`), those that drop
//! below a size (`.shrink_drop_below`), then those that collapse (a child
//! whose own line holds droppable children keeps its natural size until it
//! collapses: its line then drops all of them, as this line decides, and it
//! shrinks on from what is left). What is left is shared in proportion to the
//! shrink factors (see [`super::shrink_sharing`]).
//!
//! Because the sequence is fixed and each step frees at least the room it
//! takes away, the steps only accumulate as the line narrows: nothing that
//! gave way at one width comes back at a narrower one. When the steps free
//! more than the line needs, the line gives way again with them settled, so
//! the rest goes back to children that only trimmed, never to one that gave
//! way.

use super::shrink_sharing::share_within_order;

/// Overflow the shrinking children of an order may still have to absorb
/// before a child of that order gives way whole; keeps float noise from
/// dropping a child that fits.
const DROP_TOLERANCE: f32 = 0.01;

/// One shrinking child of the line.
#[derive(Clone, Copy)]
pub(super) struct ShrinkItem {
  pub(super) factor: f32,
  /// Main size before shrinking.
  pub(super) natural: f32,
  /// Main size this child never shrinks below. Unused for a droppable child.
  pub(super) floor: f32,
  pub(super) order: i32,
  /// Keeps its natural size or drops out (`ShrinkLimit::Drop`).
  pub(super) droppable: bool,
  /// Shrinks to its floor, then drops out when its order needs more room
  /// (`.shrink_drop_below`).
  pub(super) drops_at_floor: bool,
  /// For a child whose own line holds droppable children: it keeps its
  /// natural size (`floor`) until its order needs more room, then collapses
  /// into this.
  pub(super) collapse: Option<Collapse>,
}

/// What a collapsing child becomes: a child shrinking from `natural` (its
/// size with its droppable children gone) down to `floor`.
#[derive(Clone, Copy)]
pub(super) struct Collapse {
  pub(super) natural: f32,
  pub(super) floor: f32,
}

/// What the overflow does to one child.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ShrinkOutcome {
  Keep,
  Resize(f32),
  /// Collapsed (see [`ShrinkItem::collapse`]) and laid out at this size.
  Collapsed(f32),
  Drop,
}

/// The line the overflow is taken from.
#[derive(Clone, Copy)]
pub(super) struct LineSpace {
  pub(super) overflow: f32,
  /// Spacing between two children, freed when one of them is dropped.
  pub(super) gap: f32,
  /// Children that take up space in the line.
  pub(super) occupied: usize,
  /// Share in whole pixels (a line that uses orders or limits).
  pub(super) whole_pixels: bool,
}

/// Whether a child gave way whole.
#[derive(Clone, Copy, PartialEq)]
enum Fate {
  Stays,
  Dropped,
  Collapsed,
}

/// The outcome for every item, in the order of `items`. In a `collapsed`
/// line (its parent collapsed it) every child that can drop is dropped and
/// every child that can collapse is collapsed before anything is shared.
pub(super) fn distribute(items: &[ShrinkItem], space: LineSpace, collapsed: bool) -> Vec<ShrinkOutcome> {
  let mut settled: Vec<Fate> = items
    .iter()
    .map(|item| match item {
      _ if !collapsed => Fate::Stays,
      ShrinkItem { droppable: true, .. }
      | ShrinkItem {
        drops_at_floor: true, ..
      } => Fate::Dropped,
      ShrinkItem { collapse: Some(_), .. } => Fate::Collapsed,
      _ => Fate::Stays,
    })
    .collect();
  loop {
    let (outcomes, fates, overflow_left) = give_way_in_order(items, space, &settled);
    // When what gave way frees more than the line needed, the line gives way
    // again with it settled, so the rest goes back to children that trimmed.
    if fates == settled || overflow_left >= -DROP_TOLERANCE {
      return outcomes;
    }
    settled = fates;
  }
}

/// `item` as it shrinks once collapsed.
fn collapsed(item: ShrinkItem) -> ShrinkItem {
  match item.collapse {
    Some(collapse) => ShrinkItem {
      natural: collapse.natural,
      floor: collapse.floor,
      collapse: None,
      ..item
    },
    None => item,
  }
}

/// One round of [`distribute`] with the `settled` fates already applied: the
/// outcomes, every fate, and the overflow left at the end (negative when
/// what gave way freed more than the line needed).
fn give_way_in_order(items: &[ShrinkItem], space: LineSpace, settled: &[Fate]) -> (Vec<ShrinkOutcome>, Vec<Fate>, f32) {
  let mut fates = settled.to_vec();
  let mut shrinking_items: Vec<ShrinkItem> = items.to_vec();
  let mut step = GiveWayStep {
    overflow: space.overflow,
    occupied: space.occupied,
    gap: space.gap,
    capacity: 0.0,
  };
  for index in 0..items.len() {
    match fates[index] {
      Fate::Stays => {}
      Fate::Dropped => step.remove(items[index].natural),
      Fate::Collapsed => {
        shrinking_items[index] = collapsed(items[index]);
        step.overflow -= items[index].natural - shrinking_items[index].natural;
      }
    }
  }
  let mut orders: Vec<i32> = (0..items.len())
    .filter(|&index| fates[index] != Fate::Dropped)
    .map(|index| items[index].order)
    .collect();
  orders.sort_unstable();
  orders.dedup();

  let mut outcomes = vec![ShrinkOutcome::Keep; items.len()];
  for order in orders {
    if step.overflow <= 0.0 {
      break;
    }
    let members: Vec<usize> = (0..items.len())
      .filter(|&index| fates[index] != Fate::Dropped && items[index].order == order)
      .collect();
    let shrinking = give_way_whole(items, &members, &mut shrinking_items, &mut fates, &mut step);
    if step.overflow <= 0.0 {
      break;
    }
    step.overflow = share_within_order(
      &shrinking_items,
      &shrinking,
      step.overflow,
      space.whole_pixels,
      &mut outcomes,
    );
  }
  for (index, fate) in fates.iter().enumerate() {
    outcomes[index] = match (fate, outcomes[index]) {
      (Fate::Dropped, _) => ShrinkOutcome::Drop,
      (Fate::Collapsed, ShrinkOutcome::Resize(size)) => ShrinkOutcome::Collapsed(size),
      (Fate::Collapsed, _) => ShrinkOutcome::Collapsed(shrinking_items[index].natural),
      (Fate::Stays, outcome) => outcome,
    };
  }
  (outcomes, fates, step.overflow)
}

/// Lets the `members` of one order give way whole while its shrinking
/// children cannot absorb the overflow: those that drop, those that drop
/// below a size, then those that collapse, each last first. Returns the
/// members that are left to shrink.
fn give_way_whole(
  items: &[ShrinkItem],
  members: &[usize],
  shrinking_items: &mut [ShrinkItem],
  fates: &mut [Fate],
  step: &mut GiveWayStep,
) -> Vec<usize> {
  let (mut shrinking, droppable): (Vec<usize>, Vec<usize>) =
    members.iter().copied().partition(|&index| !items[index].droppable);
  let room = |item: ShrinkItem| item.natural - item.floor;
  step.capacity = shrinking.iter().map(|&index| room(shrinking_items[index])).sum();
  for &index in droppable.iter().rev() {
    if !step.needs_more() {
      return shrinking;
    }
    step.remove(items[index].natural);
    fates[index] = Fate::Dropped;
  }
  let below: Vec<usize> = shrinking
    .iter()
    .copied()
    .filter(|&index| items[index].drops_at_floor)
    .collect();
  for &index in below.iter().rev() {
    if !step.needs_more() {
      return shrinking;
    }
    step.remove(items[index].natural);
    step.capacity -= room(shrinking_items[index]);
    fates[index] = Fate::Dropped;
    shrinking.retain(|&kept| kept != index);
  }
  let collapsing: Vec<usize> = shrinking
    .iter()
    .copied()
    .filter(|&index| items[index].collapse.is_some() && fates[index] == Fate::Stays)
    .collect();
  for &index in collapsing.iter().rev() {
    if !step.needs_more() {
      return shrinking;
    }
    let before = shrinking_items[index];
    shrinking_items[index] = collapsed(before);
    step.overflow -= before.natural - shrinking_items[index].natural;
    step.capacity += room(shrinking_items[index]) - room(before);
    fates[index] = Fate::Collapsed;
  }
  shrinking
}

/// The overflow of a line as its children give way whole.
struct GiveWayStep {
  overflow: f32,
  occupied: usize,
  gap: f32,
  /// What the order's shrinking children can still absorb above their floors.
  capacity: f32,
}

impl GiveWayStep {
  /// Whether the overflow exceeds what the order's shrinking children can
  /// absorb.
  fn needs_more(&self) -> bool {
    self.overflow > self.capacity + DROP_TOLERANCE
  }

  /// Takes a child of main size `natural` out of the line, with its spacing.
  fn remove(&mut self, natural: f32) {
    self.overflow -= natural + if self.occupied > 1 { self.gap } else { 0.0 };
    self.occupied -= 1;
  }
}
