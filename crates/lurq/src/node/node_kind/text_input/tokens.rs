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
    self.inner.lock().token_layout = layout;
  }

  pub(crate) fn token_layout(&self) -> Option<Arc<TokenLayout>> {
    let tokens = self.tokens()?;
    let inner = self.inner.lock();
    inner
      .token_layout
      .clone()
      .filter(|layout| layout.source == tokens.source)
  }
}
