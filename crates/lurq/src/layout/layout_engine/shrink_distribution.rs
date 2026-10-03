//! Sharing a single-line Row/Column's overflow among its shrinking children.
//!
//! Children give way by order, lowest first: the overflow reaches the next
//! order only once every child of the current one is at its floor or dropped.
//! Within one order the overflow is shared in proportion to the shrink
//! factors, and a child clamped at its floor freezes and hands its unused
//! share to the others. Droppable children are dropped, last first, only while
//! the shrinking children of their order cannot absorb the rest.
//!
//! A line that uses orders or limits also shares in whole pixels: every child
//! of an order loses a whole number of pixels except the one with the largest
//! factor, which takes the fraction, so a child does not lose a sliver that
//! only truncates its text into an ellipsis. Its shares follow the overflow
//! continuously (a pixel at a time). Other lines share exactly by factor.

use std::iter;

/// Overflow the shrinking children of an order may still have to absorb
/// before a droppable child of that order is dropped; keeps float noise from
/// dropping a child that fits.
const DROP_TOLERANCE: f32 = 0.01;
/// Float noise allowed when cumulative shares are cut to whole pixels, so a
/// share of exactly n pixels computed as n - ε still counts as n.
const PIXEL_TOLERANCE: f32 = 1e-3;

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
  /// Share in whole pixels (a line that uses orders or limits).
  pub(super) whole_pixels: bool,
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
    overflow = share_within_order(items, &shrinking, overflow, space.whole_pixels, &mut outcomes);
  }
  outcomes
}

/// Shares `overflow` among the `shrinking` items of one order and returns
/// what they could not absorb (0 unless all of them reached their floor).
fn share_within_order(
  items: &[ShrinkItem],
  shrinking: &[usize],
  overflow: f32,
  whole_pixels: bool,
  outcomes: &mut [ShrinkOutcome],
) -> f32 {
  let mut remaining_overflow = overflow;
  let mut remaining_shrink: f32 = shrinking.iter().map(|&index| items[index].factor).sum();
  let mut frozen = vec![false; shrinking.len()];
  clamp_at_floors(
    items,
    shrinking,
    &mut frozen,
    &mut remaining_overflow,
    &mut remaining_shrink,
    outcomes,
  );
  let active: Vec<usize> = (0..shrinking.len())
    .filter(|&slot| !frozen[slot])
    .map(|slot| shrinking[slot])
    .collect();
  if active.is_empty() {
    return remaining_overflow;
  }
  let shares: Vec<f32> = active
    .iter()
    .map(|&index| remaining_overflow * (items[index].factor / remaining_shrink))
    .collect();
  let amounts = if whole_pixels {
    whole_pixel_amounts(items, &active, &shares, remaining_overflow)
  } else {
    shares
  };
  for (&index, amount) in active.iter().zip(amounts) {
    outcomes[index] = ShrinkOutcome::Resize((items[index].natural - amount).max(0.0));
  }
  0.0
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
    if *remaining_shrink <= 0.0 {
      // Subtracting frozen factors can lose a small factor to float
      // rounding; share by what the unfrozen items really have left.
      let unfrozen: Vec<usize> = (0..shrinking.len())
        .filter(|&slot| !frozen[slot])
        .map(|slot| shrinking[slot])
        .collect();
      *remaining_shrink = factor_sum(items, &unfrozen);
      if *remaining_shrink <= 0.0 {
        return;
      }
    }
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
    if !any_clamped || frozen.iter().all(|frozen| *frozen) {
      return;
    }
  }
}

fn factor_sum(items: &[ShrinkItem], indices: &[usize]) -> f32 {
  indices.iter().map(|&index| items[index].factor).sum()
}

/// Turns the `shares` of the `active` items (summing to `total`) into whole
/// pixels by cumulative rounding: walking the items with the carrier (the
/// largest factor, the first of equal ones) last, each loses the whole pixels
/// its running total crosses, and the carrier takes the rest, at least its own
/// share. A small change of `total` changes any item by at most a pixel.
/// What exceeds an item's room above its floor goes to the others' room,
/// the carrier first.
fn whole_pixel_amounts(items: &[ShrinkItem], active: &[usize], shares: &[f32], total: f32) -> Vec<f32> {
  let carrier = (0..active.len()).fold(0, |best, slot| {
    if items[active[slot]].factor > items[active[best]].factor {
      slot
    } else {
      best
    }
  });
  let mut amounts = vec![0.0; active.len()];
  let mut cumulative = 0.0;
  let mut assigned = 0.0;
  for slot in (0..active.len()).filter(|&slot| slot != carrier) {
    cumulative += shares[slot];
    let whole = (cumulative + PIXEL_TOLERANCE).floor().min(total);
    amounts[slot] = whole - assigned;
    assigned = whole;
  }
  amounts[carrier] = (total - assigned).max(0.0);

  let room = |slot: usize| (items[active[slot]].natural - items[active[slot]].floor).max(0.0);
  let mut excess = 0.0;
  for (slot, amount) in amounts.iter_mut().enumerate() {
    if *amount > room(slot) {
      excess += *amount - room(slot);
      *amount = room(slot);
    }
  }
  let others = (0..active.len()).filter(|&slot| slot != carrier);
  for slot in iter::once(carrier).chain(others) {
    if excess <= 0.0 {
      break;
    }
    let taken = (room(slot) - amounts[slot]).min(excess);
    amounts[slot] += taken;
    excess -= taken;
  }
  amounts
}
