//! Sharing one order's part of a line's overflow among its shrinking
//! children: in proportion to their factors, each clamped at its floor, and in
//! whole pixels for a line that uses give-way rules (see
//! [`super::shrink_distribution`]).

use std::iter;

use super::shrink_distribution::{ShrinkItem, ShrinkOutcome};

/// Float noise allowed when cumulative shares are cut to whole pixels, so a
/// share of exactly n pixels computed as n - ε still counts as n.
const PIXEL_TOLERANCE: f32 = 1e-3;

/// Shares `overflow` among the `shrinking` items of one order and returns
/// what they could not absorb (0 unless all of them reached their floor).
pub(super) fn share_within_order(
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
/// What exceeds an item's room above its floor goes to the others' room:
/// the carrier first, then items that already lose a pixel or more, so a
/// fraction lands on an item that loses nothing only when no other can take
/// it.
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
  let losing: Vec<usize> = (0..active.len())
    .filter(|&slot| slot != carrier && amounts[slot] >= 1.0)
    .collect();
  let untouched: Vec<usize> = (0..active.len())
    .filter(|&slot| slot != carrier && amounts[slot] < 1.0)
    .collect();
  for slot in iter::once(carrier).chain(losing).chain(untouched) {
    if excess <= 0.0 {
      break;
    }
    let taken = (room(slot) - amounts[slot]).min(excess);
    amounts[slot] += taken;
    excess -= taken;
  }
  amounts
}
