//! A theme for a subtree. The engine lays out and paints with the values of
//! one theme (`LayoutEngine::set_theme`); an element that carries a theme of
//! its own (`Node::theme_scope`) has that theme's values while it and what is
//! in it are laid out and painted, and the window's are back after it.

use std::sync::Arc;

use super::LayoutEngine;
use crate::{
  app::theme::{
    Theme, ThemeBorderSizes, ThemeCaret, ThemePalette, ThemeRadii, ThemeShadows, ThemeSpacing, ThemeTypography,
  },
  layout::scrollbar::ScrollBarStyle,
  node::node::Node,
};

/// The values of a theme that layout and paint read.
pub(super) struct ThemeValues {
  palette: ThemePalette,
  border_sizes: ThemeBorderSizes,
  spacing: ThemeSpacing,
  radii: ThemeRadii,
  caret: ThemeCaret,
  scrollbar: ScrollBarStyle,
  typography: ThemeTypography,
  shadows: Arc<ThemeShadows>,
  #[cfg(feature = "markdown")]
  text_marks: Arc<crate::app::theme::ThemeTextMarks>,
}

impl ThemeValues {
  fn of(theme: &Theme) -> Self {
    Self {
      palette: theme.palette().clone(),
      border_sizes: theme.border_sizes().clone(),
      spacing: theme.spacing().clone(),
      radii: theme.radii().clone(),
      caret: *theme.caret(),
      scrollbar: theme.scrollbar().clone(),
      typography: theme.typography().clone(),
      shadows: theme.shared_shadows(),
      #[cfg(feature = "markdown")]
      text_marks: theme.shared_text_marks(),
    }
  }
}

impl LayoutEngine {
  /// Puts `values` in the place of the ones the engine reads, and returns those.
  fn swap_theme_values(&self, values: ThemeValues) -> ThemeValues {
    ThemeValues {
      palette: self.palette.replace(values.palette),
      border_sizes: self.border_sizes.replace(values.border_sizes),
      spacing: self.spacing.replace(values.spacing),
      radii: self.radii.replace(values.radii),
      caret: self.caret.replace(values.caret),
      scrollbar: self.scrollbar.replace(values.scrollbar),
      typography: self.typography.replace(values.typography),
      shadows: self.shadows.replace(values.shadows),
      #[cfg(feature = "markdown")]
      text_marks: self.text_marks.replace(values.text_marks),
    }
  }

  /// Enters `node` for laying out or painting it and what is in it: if it
  /// carries a theme, the values of that theme are the ones read until
  /// [`Self::leave_theme_scope`]. Returns what to give back to it. The
  /// values are boxed: the recursion that calls this keeps a pointer in its
  /// frame, not the tables.
  pub(super) fn enter_theme_scope(&self, node: &Node) -> Option<Box<ThemeValues>> {
    let theme = node.theme_scope.as_deref()?;
    let version = theme.version();
    {
      // The engine remembers the version it laid the scope out at, so that a
      // change of the scope's theme lays the tree out again.
      let mut scopes = self.theme_scopes.borrow_mut();
      match scopes.iter_mut().find(|(seen, _)| seen.same_theme(theme)) {
        Some((_, seen)) => *seen = version,
        None => scopes.push((theme.clone(), version)),
      }
    }
    Some(Box::new(self.swap_theme_values(ThemeValues::of(theme))))
  }

  pub(super) fn leave_theme_scope(&self, previous: Option<Box<ThemeValues>>) {
    if let Some(previous) = previous {
      self.swap_theme_values(*previous);
    }
  }

  /// Whether a theme some element of the tree carries is at another version
  /// than when the tree was last laid out with it: its values may be other
  /// ones, and every node laid out with them has to be laid out again.
  pub(crate) fn theme_scopes_changed(&self) -> bool {
    self
      .theme_scopes
      .borrow()
      .iter()
      .any(|(theme, seen)| theme.version() != *seen)
  }

  /// Forgets the themes of the scopes, before a layout of every node: each element that carries one is
  /// laid out and names it again, and a theme no element carries is not watched any more.
  pub(crate) fn forget_theme_scopes(&self) {
    self.theme_scopes.borrow_mut().clear();
  }

  /// Takes the themes of the scopes as they are now for laid out: a change
  /// that did not show is not a change next time. Called when a pass that
  /// laid nothing out is done; a layout records the versions it used itself.
  pub(crate) fn settle_theme_scopes(&self) {
    for (theme, seen) in self.theme_scopes.borrow_mut().iter_mut() {
      *seen = theme.version();
    }
  }
}
