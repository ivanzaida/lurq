//! Which graphics card a renderer draws on.

/// The graphics card a renderer draws on, on a machine with more than one: a laptop with a card of its
/// processor's and one beside it. Given to an engine before its first frame
/// (`WgpuRenderEngine::with_graphics_card`, `Dx12RenderEngine::with_graphics_card`).
///
/// A window that shows the frames of a decoder is best drawn on the card that decodes: its frames are then
/// drawn where the decoder put them, and go through memory otherwise (`images::CardFrames`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GraphicsCard {
  /// The system's choice, as before there was this.
  #[default]
  Any,
  /// The card that takes the least power: the processor's own, on a laptop.
  LowPower,
  /// The fastest card: the one beside the processor, on a laptop.
  HighPerformance,
  /// The card of this identifier, by its two halves (the `LowPart` and `HighPart` of its `LUID`), as
  /// `images::card_frames_adapter()` gives them and as a decoder knows its card. A thing of Windows: on
  /// another system, and where no card of the identifier draws to the window, the system's choice is taken.
  Luid(u32, i32),
}

impl GraphicsCard {
  /// The card of the identifier `luid`, as [`Luid`](Self::Luid).
  pub fn of_luid(luid: (u32, i32)) -> Self {
    Self::Luid(luid.0, luid.1)
  }

  /// The identifier asked for, where one is.
  pub fn luid(self) -> Option<(u32, i32)> {
    match self {
      Self::Luid(low, high) => Some((low, high)),
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::GraphicsCard;

  #[test]
  fn a_card_asked_for_by_its_identifier_says_it() {
    assert_eq!(GraphicsCard::of_luid((7, -1)), GraphicsCard::Luid(7, -1));
    assert_eq!(GraphicsCard::Luid(7, -1).luid(), Some((7, -1)));
    assert_eq!(GraphicsCard::HighPerformance.luid(), None);
    assert_eq!(GraphicsCard::default(), GraphicsCard::Any);
  }
}
