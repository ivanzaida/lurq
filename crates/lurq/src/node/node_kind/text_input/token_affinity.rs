//! Spatial affinity belongs only to the exact decorated input geometry.
use super::*;
use crate::layout::layout_engine::input_token_layout::TokenLayout;

#[derive(Clone)]
pub(super) struct TokenAffinity {
  layout: Arc<TokenLayout>,
  candidate: usize,
}

impl TokenAffinity {
  pub(super) fn matches(&self, layout: Option<&Arc<TokenLayout>>) -> bool {
    layout.is_some_and(|layout| self.layout.same_geometry(layout))
  }
}

impl TextInputState {
  pub(super) fn token_hit(&self, x: f32, y: f32) -> Option<usize> {
    let layout = self.token_layout()?;
    let mut inner = self.inner.lock();
    let candidate = layout.hit(x + inner.scroll_x, y + inner.scroll_y);
    let byte = layout.carets[candidate].byte;
    inner.token_affinity = Some(TokenAffinity { layout, candidate });
    Some(byte)
  }

  pub(super) fn sync_token_caret(&self) -> bool {
    let Some(layout) = self.token_layout() else {
      return false;
    };
    let mut inner = self.inner.lock();
    let candidate = retained_candidate(&inner, &layout);
    let position = layout.carets[candidate];
    inner.caret_x = position.x;
    inner.caret_y = position.y;
    inner.caret_height = position.height;
    let retained = inner.token_affinity.as_ref().is_some_and(|affinity| {
      affinity.layout.same_geometry(&layout)
        && layout
          .carets
          .get(affinity.candidate)
          .is_some_and(|caret| caret.byte == inner.caret)
    });
    inner.token_affinity = if retained {
      Some(TokenAffinity { layout, candidate })
    } else {
      None
    };
    true
  }

  pub(crate) fn caret_font_size(&self, ordinary: f32) -> f32 {
    let Some(layout) = self.token_layout() else {
      return ordinary;
    };
    let inner = self.inner.lock();
    layout.carets[retained_candidate(&inner, &layout)].font_size
  }

  pub(crate) fn selection_band_for_row(&self, y: f32, ordinary: (f32, f32)) -> (f32, f32) {
    let Some(layout) = self.token_layout() else {
      return ordinary;
    };
    layout.selection_band(y + self.scroll_y())
  }

  pub(super) fn move_token_vertical(&self, down: bool, selecting: bool) -> bool {
    let Some(layout) = self.token_layout() else {
      return false;
    };
    let mut inner = self.inner.lock();
    let current = retained_candidate(&inner, &layout);
    let candidate = layout.vertical(current, down);
    if selecting && inner.selection_anchor.is_none() {
      inner.selection_anchor = Some(inner.caret);
    }
    if !selecting {
      inner.selection_anchor = None;
    }
    inner.caret = layout.carets[candidate].byte;
    inner.token_affinity = Some(TokenAffinity { layout, candidate });
    drop(inner);
    self.mark_layout_dirty();
    true
  }
}

fn retained_candidate(inner: &TextInputInner, layout: &Arc<TokenLayout>) -> usize {
  inner
    .token_affinity
    .as_ref()
    .filter(|affinity| affinity.layout.same_geometry(layout))
    .and_then(|affinity| {
      layout
        .carets
        .get(affinity.candidate)
        .filter(|caret| caret.byte == inner.caret)
        .map(|_| affinity.candidate)
    })
    .unwrap_or_else(|| layout.default_caret(inner.caret))
}
