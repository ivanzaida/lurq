use super::*;
use crate::{components::InputTokens, layout::layout_engine::input_token_layout::TokenLayout};

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
    self.mark_layout_dirty();
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

  pub(crate) fn token_layout(&self) -> Option<Arc<TokenLayout>> {
    let tokens = self.tokens()?;
    let inner = self.inner.lock();
    inner
      .token_layout
      .clone()
      .filter(|layout| layout.config.source == tokens.source)
  }
}
