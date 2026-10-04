//! A CPU render engine for capture tests: it paints each glyph's rect in
//! [`GLYPH_PAINT`] over opaque black, ignores everything else, and delivers a
//! requested capture through the shared capture pipeline as a GPU engine does.

use std::sync::{Arc, Mutex};

use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::{
  app::render_engine::{RenderEngine, RenderFrameCapture},
  layout::render_list::RenderList,
};

pub(crate) const GLYPH_PAINT: [u8; 4] = [255, 255, 255, 255];
const BACKGROUND: [u8; 4] = [0, 0, 0, 255];

/// What the engine last put on screen: RGBA8 rows `width` pixels long.
#[derive(Clone, Default)]
pub(crate) struct ScreenFrame {
  pub width: u32,
  pub rgba: Vec<u8>,
}

impl ScreenFrame {
  pub(crate) fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
    let index = (y * self.width + x) as usize * 4;
    self.rgba[index..index + 4].try_into().expect("four channels")
  }
}

pub(crate) struct GlyphPainter {
  width: u32,
  height: u32,
  screen: Arc<Mutex<ScreenFrame>>,
}

impl GlyphPainter {
  /// An engine factory for `Tree::set_render_engine_factory`, and the frame
  /// its engine shows.
  pub(crate) fn factory() -> (impl Fn() -> Box<dyn RenderEngine> + 'static, Arc<Mutex<ScreenFrame>>) {
    let screen = Arc::new(Mutex::new(ScreenFrame::default()));
    let shown = screen.clone();
    let factory = move || {
      Box::new(Self {
        width: 0,
        height: 0,
        screen: shown.clone(),
      }) as Box<dyn RenderEngine>
    };
    (factory, screen)
  }

  fn paint(&self, list: &RenderList) -> Vec<u8> {
    let mut rgba = BACKGROUND.repeat((self.width * self.height) as usize);
    for glyph in &list.glyphs {
      let x0 = glyph.x.floor().max(0.0) as u32;
      let y0 = glyph.y.floor().max(0.0) as u32;
      let x1 = ((glyph.x + glyph.width).ceil().max(0.0) as u32).min(self.width);
      let y1 = ((glyph.y + glyph.height).ceil().max(0.0) as u32).min(self.height);
      for y in y0..y1 {
        for x in x0..x1 {
          let index = (y * self.width + x) as usize * 4;
          rgba[index..index + 4].copy_from_slice(&GLYPH_PAINT);
        }
      }
    }
    rgba
  }
}

impl RenderEngine for GlyphPainter {
  fn resize(&mut self, width: u32, height: u32) {
    self.width = width;
    self.height = height;
  }

  fn render(&mut self, list: &RenderList, _: WindowHandle<'_>, _: DisplayHandle<'_>) -> bool {
    let rgba = self.paint(list);
    *self.screen.lock().unwrap() = ScreenFrame {
      width: self.width,
      rgba,
    };
    true
  }

  fn supports_frame_capture(&self) -> bool {
    true
  }

  fn render_with_capture(
    &mut self,
    list: &RenderList,
    window: WindowHandle<'_>,
    display: DisplayHandle<'_>,
    capture: Option<RenderFrameCapture>,
  ) -> bool {
    self.render(list, window, display);
    if let Some(capture) = capture {
      let screen = self.screen.lock().unwrap().clone();
      let mut pixels = Vec::with_capacity((capture.width * capture.height) as usize * 4);
      for y in capture.y..capture.y + capture.height {
        for x in capture.x..capture.x + capture.width {
          pixels.extend_from_slice(&screen.pixel(x, y));
        }
      }
      crate::app::frame_capture::finish_capture(pixels, &capture);
    }
    true
  }
}

/// A surface with a window handle, so a pass renders; the engines above
/// never use it.
pub(crate) struct TestSurface;

impl raw_window_handle::HasWindowHandle for TestSurface {
  fn window_handle(&self) -> Result<WindowHandle<'_>, raw_window_handle::HandleError> {
    let handle = raw_window_handle::Win32WindowHandle::new(std::num::NonZeroIsize::new(1).expect("non-zero"));
    // SAFETY: the handle is never dereferenced: the test engines ignore it.
    Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
  }
}

impl raw_window_handle::HasDisplayHandle for TestSurface {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, raw_window_handle::HandleError> {
    // SAFETY: as above, the handle is never used.
    Ok(unsafe { DisplayHandle::borrow_raw(raw_window_handle::WindowsDisplayHandle::new().into()) })
  }
}
