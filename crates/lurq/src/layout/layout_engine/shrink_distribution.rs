//! Sharing a single-line Row/Column's overflow among its shrinking children.
//!
//! Children give way by order, lowest first: the overflow reaches the next
//! order only once every child of the current one is at its floor or dropped.
//! Within one order the overflow is shared in proportion to the shrink
//! factors: a child clamped at its floor freezes and hands its unused share
//! to the others, a share under one pixel goes to the others as well (so no
//! child loses a sliver that only truncates its text into an ellipsis), and
//! droppable children are dropped, last first, only while the shrinking
//! children of the order cannot absorb the rest.

/// A share smaller than this goes to the other children of the order.
const MIN_SHARE: f32 = 1.0;
/// Overflow the shrinking children of an order may still have to absorb
/// before a droppable child of that order is dropped; keeps float noise from
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
}

/// What the overflow does to one child.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ShrinkOutcome {
  Keep,
  Resize(f32),
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
}

/// The outcome for every item, in the order of `items`.
pub(super) fn distribute(items: &[ShrinkItem], space: LineSpace) -> Vec<ShrinkOutcome> {
  let mut outcomes = vec![ShrinkOutcome::Keep; items.len()];
  let mut orders: Vec<i32> = items.iter().map(|item| item.order).collect();
  orders.sort_unstable();
  orders.dedup();

  let mut overflow = space.overflow;
  let mut occupied = space.occupied;
  for order in orders {
    if overflow <= 0.0 {
      break;
    }
    let members: Vec<usize> = (0..items.len()).filter(|&index| items[index].order == order).collect();
    let (shrinking, droppable): (Vec<usize>, Vec<usize>) =
      members.into_iter().partition(|&index| !items[index].droppable);
    let capacity: f32 = shrinking
      .iter()
      .map(|&index| items[index].natural - items[index].floor)
      .sum();
    for &index in droppable.iter().rev() {
      if overflow <= capacity + DROP_TOLERANCE {
        break;
      }
      overflow -= items[index].natural + if occupied > 1 { space.gap } else { 0.0 };
      occupied -= 1;
      outcomes[index] = ShrinkOutcome::Drop;
    }
    if overflow <= 0.0 {
      break;
    }
    overflow = share_within_order(items, &shrinking, overflow, &mut outcomes);
  }
  outcomes
}

/// Shares `overflow` among the `shrinking` items of one order and returns
/// what they could not absorb (0 unless all of them reached their floor).
fn share_within_order(items: &[ShrinkItem], shrinking: &[usize], overflow: f32, outcomes: &mut [ShrinkOutcome]) -> f32 {
  let mut remaining_overflow = overflow;
  let mut remaining_shrink: f32 = shrinking.iter().map(|&index| items[index].factor).sum();
  let mut frozen = vec![false; shrinking.len()];
  loop {
    clamp_at_floors(
      items,
      shrinking,
      &mut frozen,
      &mut remaining_overflow,
      &mut remaining_shrink,
      outcomes,
    );
    let active: Vec<usize> = (0..shrinking.len()).filter(|&slot| !frozen[slot]).collect();
    if active.is_empty() {
      return remaining_overflow;
    }
    let share = |slot: usize| remaining_overflow * (items[shrinking[slot]].factor / remaining_shrink);
    let slivers = slivers(&active, share);
    if slivers.is_empty() {
      for slot in active {
        let item = items[shrinking[slot]];
        outcomes[shrinking[slot]] = ShrinkOutcome::Resize((item.natural - share(slot)).max(0.0));
      }
      return 0.0;
    }
    for slot in slivers {
      frozen[slot] = true;
      remaining_shrink -= items[shrinking[slot]].factor;
    }
  }
}

/// Freezes every item whose share would take it below its floor at that
/// floor, re-sharing until no further item clamps.
fn clamp_at_floors(
  items: &[ShrinkItem],
  shrinking: &[usize],
  frozen: &mut [bool],
  remaining_overflow: &mut f32,
  remaining_shrink: &mut f32,
  outcomes: &mut [ShrinkOutcome],
) {
  loop {
    let mut any_clamped = false;
    for (slot, &index) in shrinking.iter().enumerate() {
      if frozen[slot] {
        continue;
      }
      let item = items[index];
      let shrink_amount = *remaining_overflow * (item.factor / *remaining_shrink);
      let new_main = (item.natural - shrink_amount).max(item.floor);
      if new_main > item.natural - shrink_amount {
        frozen[slot] = true;
        *remaining_overflow -= item.natural - new_main;
        *remaining_shrink -= item.factor;
        any_clamped = true;
        outcomes[index] = ShrinkOutcome::Resize(new_main);
      }
    }
    if !any_clamped || *remaining_shrink <= 0.0 {
      return;
    }
  }
}

/// The active slots whose share is a positive sliver under [`MIN_SHARE`],
/// except the one with the largest share (the first of equal ones), which
/// takes what is left when every share is a sliver.
fn slivers(active: &[usize], share: impl Fn(usize) -> f32) -> Vec<usize> {
  let Some(&first) = active.first() else {
    return Vec::new();
  };
  let largest = active
    .iter()
    .copied()
    .fold(first, |best, slot| if share(slot) > share(best) { slot } else { best });
  active
    .iter()
    .copied()
    .filter(|&slot| slot != largest && share(slot) > 0.0 && share(slot) < MIN_SHARE)
    .collect()
}
