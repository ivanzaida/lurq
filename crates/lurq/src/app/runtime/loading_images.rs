//! Images decoded at runtime, resolved against the decoded-image cache.
//!
//! A node built with [`Image::from_bytes`](crate::components::Image::from_bytes)
//! or [`Image::from_file`](crate::components::Image::from_file) starts as a
//! `LoadingImage` showing its placeholder. A pass asks the cache for its
//! key: a ready image turns the node into an `Image` node (the component
//! need not render again), a failed decode puts the error view in its
//! place, and a loading one is remembered so the decode's end asks for the
//! next pass. The decode's worker wakes the window's loop when it ends.
//!
//! The walk over the tree is skipped when it cannot find anything: no
//! decode is pending for this tree, and no loading-image node was built
//! since the last walk (a counter every such node bumps).

use std::sync::atomic::{AtomicU64, Ordering};

use super::Tree;
use crate::{
  app::window::WindowWaker,
  core::IdGenerator,
  images::{ImageDecodeError, ImageKey, ImageLoadState, ImageSizeCap, ImageSource, decoded_images},
  layout::{Size, layout_kind::LayoutKind},
  node::{node::Node, node_kind::NodeKind},
  resources::{Load, LoadState, Loader},
};

/// How many loading-image nodes have been built in the process.
static LOADING_IMAGES_BUILT: AtomicU64 = AtomicU64::new(0);

/// A loading-image node was built: the next pass of every tree walks for it.
pub(crate) fn note_loading_image_built() {
  LOADING_IMAGES_BUILT.fetch_add(1, Ordering::AcqRel);
}

impl Tree {
  /// Resolves the loading images of the tree: `true` when a node changed
  /// and the layout must run again.
  pub(super) fn resolve_loading_images(&mut self, loader: &Loader) -> bool {
    // What nobody waits for any more is let go: the loads of nodes that are gone.
    loader.sweep();
    let built = LOADING_IMAGES_BUILT.load(Ordering::Acquire);
    if self.pending_images.is_empty() && self.pending_loads.is_empty() && self.loading_images_built_seen == built {
      return false;
    }
    self.loading_images_built_seen = built;
    let Some(root) = &mut self.root else {
      return false;
    };
    // One waker for the tree's window: the cache keeps one per entry and
    // window, and tells them apart by identity.
    let wake = self
      .loading_image_waker
      .get_or_insert_with(|| self.window.loop_waker())
      .clone();
    let mut pass = Pass {
      wake: &wake,
      id_gen: &self.id_gen,
      loader,
      pending: Vec::new(),
      loads: Vec::new(),
    };
    let changed = resolve_recursive(root, &mut pass);
    self.pending_images = pass.pending;
    self.pending_loads = pass.loads;
    if changed {
      self.needs_redraw = true;
    }
    changed
  }

  /// A pending decode that ended asks for a pass; called each turn of the
  /// shell's loop.
  pub(super) fn notice_decoded_images(&mut self) {
    if self.pending_images.is_empty() && self.pending_loads.is_empty() {
      return;
    }
    let cache = decoded_images();
    let ended = self
      .pending_images
      .iter()
      .any(|(key, cap)| !cache.state(*key, *cap).is_some_and(|state| state.is_loading()));
    // A load that ended, or came further than the node last showed.
    let moved = self.pending_loads.iter().any(|(load, seen)| match load.state() {
      LoadState::Loading { loaded, .. } => loaded != *seen,
      _ => true,
    });
    if ended || moved {
      self.needs_redraw = true;
    }
  }
}

/// What the walk over the loading images carries down the tree, and what it gathers.
struct Pass<'a> {
  /// Wakes the tree's window when a decode or a load ends, and as a load comes further.
  wake: &'a WindowWaker,
  id_gen: &'a IdGenerator,
  loader: &'a Loader,
  /// The decodes still running.
  pending: Vec<(ImageKey, Option<ImageSizeCap>)>,
  /// The loads still on their way, each with the bytes its node last showed.
  loads: Vec<(Load, u64)>,
}

/// Brings on the load of an image whose bytes the app's loader reads first. While they load the node shows its
/// placeholder, or how far the load has come where the loader knows its length; a load that failed shows the
/// error view a failed decode shows, with its reason. Once the bytes are there, or its pixels are in the cache
/// already, the node is decoded like any other. `true` where its children were built anew.
fn resolve_load(node: &mut Node, pass: &mut Pass<'_>) -> bool {
  let NodeKind::LoadingImage { request } = &mut node.node_kind else {
    return false;
  };
  let Some(source) = request.load_from.clone() else {
    return false;
  };
  if request.loaded && !request.failed {
    return false;
  }
  let mut rebuilt = false;
  // What a person asked for again is loaded again, whether it was the load that failed or the decode of what
  // it brought: the loader forgets what it holds of the source. A second node of the source finds the load on
  // its way and joins it.
  let retries = crate::images::load_retries(source.cache_key());
  let retried = request.failed && retries != request.retries_seen;
  request.retries_seen = retries;
  if retried {
    pass.loader.forget(&source);
    request.failed = false;
    request.loaded = false;
    request.progress_seen = 0;
    request.load = None;
    node.children = request
      .placeholder_view
      .as_ref()
      .map(|view| view().into_node())
      .into_iter()
      .collect();
    rebuilt = true;
  }
  if !request.failed && !request.loaded {
    if decoded_images().state(request.key, request.cap).is_some() {
      // Its pixels are there, or on their way: nothing to load.
      request.loaded = true;
      request.load = None;
    } else {
      let load = request.load.get_or_insert_with(|| pass.loader.load(&source));
      match load.state() {
        LoadState::Ready(bytes) => {
          request.source = ImageSource::Bytes(bytes);
          request.loaded = true;
          request.load = None;
        }
        LoadState::Failed(error) => {
          request.failed = true;
          request.load = None;
          let error = ImageDecodeError::new(error.to_string());
          node.children = request
            .error_view
            .as_ref()
            .map(|view| view(&error).into_node())
            .into_iter()
            .collect();
          rebuilt = true;
        }
        LoadState::Loading { loaded, total } => {
          load.wake_with(pass.wake.clone());
          if let (Some(view), Some(total)) = (&request.progress_view, total)
            && loaded != request.progress_seen
          {
            request.progress_seen = loaded;
            node.children = vec![view(loaded, total).into_node()];
            rebuilt = true;
          }
          pass.loads.push((load.clone(), request.progress_seen));
        }
      }
    }
  }
  if rebuilt {
    // What a view builds here was in no render: its nodes are named now, so that an element in it is found by
    // its id and not taken for the root.
    for child in &mut node.children {
      child.assign_ids(pass.id_gen);
    }
  }
  rebuilt
}

fn resolve_recursive(node: &mut Node, pass: &mut Pass<'_>) -> bool {
  #[cfg(test)]
  crate::core::layout_dirty::visit();
  if !node.prepare_layout_tracking().loading_images {
    return false;
  }
  let mut layout_dirty = resolve_load(node, pass);
  let (wake, id_gen) = (pass.wake, pass.id_gen);
  let pending = &mut pass.pending;
  if let NodeKind::LoadingImage { request } = &mut node.node_kind
    // Bytes the loader reads are decoded once they are there.
    && (request.load_from.is_none() || request.loaded)
  {
    let state = decoded_images().load(request.key, request.cap, || request.source.clone(), Some(wake.clone()));
    match state {
      ImageLoadState::Loading => {
        // Asked for anew after it failed: the placeholder again, where the node can build it.
        if request.failed {
          request.failed = false;
          node.children = request
            .placeholder_view
            .as_ref()
            .map(|view| view().into_node())
            .into_iter()
            .collect();
          for child in &mut node.children {
            child.assign_ids(id_gen);
          }
          layout_dirty = true;
        }
        pending.push((request.key, request.cap));
      }
      ImageLoadState::Ready(data) => {
        let fit = request.fit;
        node.intrinsic_size = Some(Size::new(data.width() as f32, data.height() as f32));
        node.layout_kind = LayoutKind::Leaf;
        node.children.clear();
        node.node_kind = NodeKind::Image { data, fit };
        layout_dirty = true;
      }
      ImageLoadState::Failed(error) => {
        if !request.failed {
          request.failed = true;
          node.children = request
            .error_view
            .as_ref()
            .map(|view| view(&error).into_node())
            .into_iter()
            .collect();
          // What a view builds here was in no render: its nodes are named now, so that an element in it (the
          // button that asks again) is found by its id and not taken for the root.
          for child in &mut node.children {
            child.assign_ids(id_gen);
          }
          layout_dirty = true;
        }
      }
    }
  }
  for child in &mut node.children {
    if resolve_recursive(child, pass) {
      layout_dirty = true;
    }
  }
  if layout_dirty {
    node.layout_cache.invalidate();
  }
  layout_dirty
}
