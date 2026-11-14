use std::ops::Range;

use super::*;
use crate::{
  app::events::KeyboardEvent, components::InputTokens, core::NodeId,
  layout::layout_engine::input_token_layout::TokenLayout,
};

/// Where a caret inside an atomic token goes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Toward {
  Start,
  End,
  /// The edge it is nearer.
  Nearest,
  /// The edge away from the selection's anchor: a selection that is dragged grows over the token.
  AwayFromAnchor,
}

/// What a point in the text is on: a token, and whether on its icon.
pub(super) struct TokenPoint {
  index: usize,
  range: Range<usize>,
  atomic: bool,
  on_removing_icon: bool,
}

impl TextInputState {
  pub(crate) fn set_tokens(&self, tokens: InputTokens) {
    let mut inner = self.inner.lock();
    if inner.tokens.as_ref() == Some(&tokens) {
      return;
    }
    inner.tokens = Some(tokens);
    inner.token_layout = None;
    inner.token_affinity = None;
    drop(inner);
    self.mark_layout_dirty_keeping_scroll();
  }

  pub(crate) fn tokens(&self) -> Option<InputTokens> {
    let inner = self.inner.lock();
    if inner.mask.is_some() || inner.composition.is_some() {
      return None;
    }
    inner
      .tokens
      .clone()
      .filter(|tokens| tokens.source.as_ref() == self.value())
  }

  pub(crate) fn set_token_layout(&self, layout: Option<Arc<TokenLayout>>) {
    let mut inner = self.inner.lock();
    if inner
      .token_affinity
      .as_ref()
      .is_some_and(|affinity| !affinity.matches(layout.as_ref()))
    {
      inner.token_affinity = None;
    }
    inner.token_layout = layout;
  }

  /// The decorated layout that editing and hit testing use: the one laid
  /// out over the current value, never while composing or masked.
  pub(crate) fn token_layout(&self) -> Option<Arc<TokenLayout>> {
    let value = self.value();
    let inner = self.inner.lock();
    if inner.mask.is_some() || inner.composition.is_some() {
      return None;
    }
    inner
      .token_layout
      .clone()
      .filter(|layout| layout.config.source.as_ref() == value)
  }

  /// The decorated layout that paint draws: also a styled input's while it
  /// composes, laid out over the displayed text.
  pub(crate) fn painted_layout(&self) -> Option<Arc<TokenLayout>> {
    let shown = self.composed_text().unwrap_or_else(|| self.value());
    let inner = self.inner.lock();
    if inner.mask.is_some() {
      return None;
    }
    inner
      .token_layout
      .clone()
      .filter(|layout| layout.config.source.as_ref() == shown)
  }
}

impl TextInputState {
  /// Whether `other` is this state, and not one like it.
  pub(crate) fn same_state(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }

  /// Whether the pointer is on the icon of a token that a click takes away: what is pressed there is a button.
  pub(crate) fn token_icon_hovered(&self) -> bool {
    self.inner.lock().token_icon_hovered
  }

  /// The token the pointer is on, where its style has a look for that.
  pub(crate) fn hovered_token(&self) -> Option<usize> {
    self.inner.lock().hovered_token
  }

  /// The selection's start and end, where it is not empty.
  pub(crate) fn selection_bounds(&self) -> Option<(usize, usize)> {
    let value = self.value();
    let inner = self.inner.lock();
    selection_range_indices(&value, inner.selection_anchor, inner.caret)
  }

  /// The token whose fill is at `(x, y)`, in the coordinates the caret is placed by, and whether the point
  /// is on an icon of it that takes it away. A token's icon is at the end of its last fragment.
  pub(super) fn token_at_point(&self, x: f32, y: f32) -> Option<TokenPoint> {
    let tokens = self.tokens()?;
    let layout = self.token_layout()?;
    if layout.config.styles.len() != tokens.tokens.len() {
      return None;
    }
    let (x, y) = {
      let inner = self.inner.lock();
      (x + inner.scroll_x, y + inner.scroll_y())
    };
    let inside =
      |left: f32, top: f32, width: f32, height: f32| x >= left && x <= left + width && y >= top && y <= top + height;
    let (index, part) = layout.fragments.iter().enumerate().find_map(|(at, part)| {
      let (token, _) = part.token?;
      inside(part.x, part.y, part.width, part.height).then_some((token, at))
    })?;
    let token = &tokens.tokens[index];
    let last = layout
      .fragments
      .iter()
      .rposition(|part| part.token.is_some_and(|(token, _)| token == index));
    let on_removing_icon = token.style.icon.as_ref().is_some_and(|icon| icon.removes)
      && last == Some(part)
      && layout.config.styles[index].chip.icon.as_ref().is_some_and(|icon| {
        let part = &layout.fragments[part];
        // The icon's square, and a pixel around it.
        let left = part.x + part.width - icon.inset - icon.size;
        let top = part.y + (part.height - icon.size) * 0.5;
        inside(left - 1.0, top - 1.0, icon.size + 2.0, icon.size + 2.0)
      });
    Some(TokenPoint {
      index,
      range: token.range.clone(),
      atomic: token.atomic,
      on_removing_icon,
    })
  }

  /// Notes the token the pointer is on, at `point` in the coordinates the caret is placed by, or none. Only
  /// a token with a look for it is noted, and a change has the input drawn again.
  pub(crate) fn hover_token_at_point(&self, point: Option<(f32, f32)>) {
    let under = point.and_then(|(x, y)| self.token_at_point(x, y));
    let on_icon = under.as_ref().is_some_and(|token| token.on_removing_icon);
    let hovered = under
      .filter(|token| {
        self
          .tokens()
          .is_some_and(|tokens| tokens.tokens[token.index].style.hovered.is_some())
      })
      .map(|token| token.index);
    let mut inner = self.inner.lock();
    inner.token_icon_hovered = on_icon;
    if inner.hovered_token != hovered {
      inner.hovered_token = hovered;
      drop(inner);
      self.mark_layout_dirty_keeping_scroll();
    }
  }

  /// The atomic token that `byte` is strictly inside of.
  fn atomic_around(&self, byte: usize) -> Option<Range<usize>> {
    let tokens = self.tokens()?;
    tokens
      .tokens
      .iter()
      .find(|token| token.atomic && token.range.start < byte && byte < token.range.end)
      .map(|token| token.range.clone())
  }

  /// Moves the caret, and the anchor of a selection, out of the atomic tokens they are inside of: the
  /// caret `toward` an edge, the anchor to the edge away from the caret, so that the selection takes all
  /// of a token it takes part of.
  pub(super) fn keep_out_of_tokens(&self, toward: Toward) {
    let (caret, anchor) = {
      let inner = self.inner.lock();
      (inner.caret, inner.selection_anchor)
    };
    let caret = match self.atomic_around(caret) {
      Some(range) => match toward {
        Toward::Start => range.start,
        Toward::End => range.end,
        Toward::AwayFromAnchor if anchor.is_some() => {
          if anchor.is_some_and(|anchor| anchor <= range.start) {
            range.end
          } else {
            range.start
          }
        }
        _ if caret - range.start <= range.end - caret => range.start,
        _ => range.end,
      },
      None => caret,
    };
    let anchor = anchor.map(|anchor| match self.atomic_around(anchor) {
      Some(range) if caret >= range.end => range.start,
      Some(range) => range.end,
      None => anchor,
    });
    let mut inner = self.inner.lock();
    inner.caret = caret;
    inner.selection_anchor = anchor;
  }

  /// Selects all of the token at `(x, y)` where it is atomic; `false` where there is none.
  pub(super) fn select_atomic_at_point(&self, x: f32, y: f32) -> bool {
    let Some(token) = self.token_at_point(x, y).filter(|token| token.atomic) else {
      return false;
    };
    let mut inner = self.inner.lock();
    inner.selection_anchor = Some(token.range.start);
    inner.caret = token.range.end;
    drop(inner);
    self.mark_layout_dirty();
    true
  }

  /// Takes away the token whose removing icon is at `(x, y)`: its text is deleted as an edit, through
  /// `on_input`, as one step of the undo, with the space after it where it starts the text or follows a
  /// space, so that taking a word away from between two leaves one space. `false` where the point is on no
  /// such icon.
  pub(super) fn remove_token_at_point(&self, x: f32, y: f32) -> bool {
    let Some(token) = self.token_at_point(x, y).filter(|token| token.on_removing_icon) else {
      return false;
    };
    let value = self.value();
    let alone = value[..token.range.start]
      .chars()
      .next_back()
      .is_none_or(char::is_whitespace);
    let end = match value[token.range.end..].chars().next() {
      Some(' ') if alone => token.range.end + 1,
      _ => token.range.end,
    };
    {
      let mut inner = self.inner.lock();
      inner.selection_anchor = Some(token.range.start);
      inner.caret = end;
      inner.hovered_token = None;
    }
    let keyboard = KeyboardEvent::new("Delete", "Delete", false, false, false, false, NodeId::UNASSIGNED);
    self.delete(&keyboard);
    true
  }

  /// Selects the atomic token that ends at the caret (`backward`) or starts at it, where there is one and
  /// nothing is selected: the first Backspace or Delete at a token selects it, the next deletes it.
  pub(super) fn select_atomic_beside_caret(&self, backward: bool) -> bool {
    if self.has_selection() {
      return false;
    }
    let Some(tokens) = self.tokens() else {
      return false;
    };
    let caret = self.inner.lock().caret;
    let Some(range) = tokens
      .tokens
      .iter()
      .find(|token| {
        token.atomic
          && if backward {
            token.range.end == caret
          } else {
            token.range.start == caret
          }
      })
      .map(|token| token.range.clone())
    else {
      return false;
    };
    let mut inner = self.inner.lock();
    inner.selection_anchor = Some(if backward { range.end } else { range.start });
    inner.caret = if backward { range.start } else { range.end };
    drop(inner);
    self.mark_layout_dirty();
    true
  }
}
