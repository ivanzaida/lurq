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

  /// Gives the input `source`, asked for the tokens of each value it holds; what it gave is kept while the value
  /// is the same.
  pub(crate) fn set_token_source(&self, source: super::TokenSource) {
    let mut inner = self.inner.lock();
    inner.token_source = Some(source);
    inner.tokens = None;
    inner.token_layout = None;
    inner.token_affinity = None;
    drop(inner);
    self.mark_layout_dirty_keeping_scroll();
  }

  pub(crate) fn tokens(&self) -> Option<InputTokens> {
    let value = self.value();
    let source = {
      let inner = self.inner.lock();
      if inner.mask.is_some() || inner.composition.is_some() {
        return None;
      }
      if let Some(tokens) = inner.tokens.as_ref().filter(|tokens| tokens.source.as_ref() == value) {
        return Some(tokens.clone());
      }
      inner.token_source.clone()?
    };
    // Asked with the input let go: what gives the tokens may read anything.
    let tokens = source(&value);
    let mut inner = self.inner.lock();
    inner.tokens = Some(tokens.clone());
    inner.token_layout = None;
    inner.token_affinity = None;
    Some(tokens).filter(|tokens| tokens.source.as_ref() == value)
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

  /// Has the layout put `anchor` where the byte `byte` of the value stands, or no element.
  pub(crate) fn set_byte_anchor(&self, anchor: Option<(crate::core::ElementRef, usize)>) {
    self.inner.lock().byte_anchor = anchor;
  }

  /// The element the layout puts at a byte of the value, and the byte.
  pub(crate) fn byte_anchor(&self) -> Option<(crate::core::ElementRef, usize)> {
    self.inner.lock().byte_anchor.clone()
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
    self.hover_object_handle(point);
    let under = point.and_then(|(x, y)| self.token_at_point(x, y));
    let on_icon = under.as_ref().is_some_and(|token| token.on_removing_icon);
    let hovered = under
      .filter(|token| {
        self
          .tokens()
          .is_some_and(|tokens| tokens.tokens[token.index].style.hovered.is_some())
      })
      .map(|token| token.index);
    // How long the look under the pointer takes to come and go, where a token's style says so.
    let length = self.tokens().map_or(std::time::Duration::ZERO, |tokens| {
      tokens
        .tokens
        .iter()
        .filter_map(|token| token.style.fade)
        .max()
        .unwrap_or_default()
    });
    let mut inner = self.inner.lock();
    inner.token_icon_hovered = on_icon;
    if inner.hovered_token != hovered {
      let from = inner.hovered_token;
      inner.hovered_token = hovered;
      let fades = !length.is_zero();
      inner.token_fade = fades.then_some(super::token_fade::TokenFade { from, start: None });
      inner.token_fade_length = length;
      drop(inner);
      if fades {
        super::token_fade::follow(self);
      }
      self.mark_layout_dirty_keeping_scroll();
    }
  }

  /// The change of the look under the pointer on its way, as the frame drawn at `now` lays it out, and how long it
  /// takes: the first layout of it is where it starts.
  pub(crate) fn token_fade_at(
    &self,
    now: crate::time::Instant,
  ) -> (Option<super::token_fade::TokenFade>, std::time::Duration) {
    let mut inner = self.inner.lock();
    if let Some(fade) = &mut inner.token_fade
      && fade.start.is_none()
    {
      fade.start = Some(now);
    }
    (inner.token_fade, inner.token_fade_length)
  }

  /// Whether the change of the look under the pointer is still on its way at `now`; once it is over it is let go.
  pub(super) fn token_fade_going(&self, now: crate::time::Instant) -> bool {
    let mut inner = self.inner.lock();
    let length = inner.token_fade_length;
    let going = inner.token_fade.is_some_and(|fade| {
      fade
        .start
        .is_none_or(|start| now.saturating_duration_since(start) < length)
    });
    if !going {
      inner.token_fade = None;
    }
    going
  }

  /// What is one thing to the caret, as bytes of the value: the objects of a styled input, the atomic tokens of
  /// an input with tokens.
  fn atomic_ranges(&self) -> Vec<Range<usize>> {
    let objects = self.object_ranges();
    if !objects.is_empty() {
      return objects;
    }
    self
      .tokens()
      .map(|tokens| {
        tokens
          .tokens
          .iter()
          .filter(|token| token.atomic)
          .map(|token| token.range.clone())
          .collect()
      })
      .unwrap_or_default()
  }

  /// The atomic token or the object that `byte` is strictly inside of.
  fn atomic_around(&self, byte: usize) -> Option<Range<usize>> {
    self
      .atomic_ranges()
      .into_iter()
      .find(|range| range.start < byte && byte < range.end)
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

  /// Whether `(x, y)` is on the icon of a token that a click takes away.
  pub(super) fn on_removing_icon(&self, x: f32, y: f32) -> bool {
    self.token_at_point(x, y).is_some_and(|token| token.on_removing_icon)
  }

  /// Selects all of the token at `(x, y)` where it is atomic, or all of the object whose box is there; `false`
  /// where there is neither.
  pub(super) fn select_atomic_at_point(&self, x: f32, y: f32) -> bool {
    let token = self
      .token_at_point(x, y)
      .filter(|token| token.atomic)
      .map(|token| token.range);
    let Some(range) = token.or_else(|| self.object_at_point(x, y)) else {
      return false;
    };
    let mut inner = self.inner.lock();
    inner.selection_anchor = Some(range.start);
    inner.caret = range.end;
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
    let caret = self.inner.lock().caret;
    let Some(range) = self.atomic_ranges().into_iter().find(|range| {
      if backward {
        range.end == caret
      } else {
        range.start == caret
      }
    }) else {
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
