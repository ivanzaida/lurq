//! End-to-end box-shadow check on a hidden Win32 surface: a themed scene goes
//! through layout and both native backends, and the two captures must match.
//! cargo run -p lurq --example box_shadow_capture_check --features screenshot,wgpu,dx12
//! Writes target/box-shadow-wgpu.png and target/box-shadow-dx12.png.
#[cfg(windows)]
fn main() {
  use std::{
    num::NonZeroIsize,
    sync::{Arc, mpsc},
    time::Duration,
  };

  use lurq::{
    app::{
      App, Tree,
      render_engine::{CapturedFrame, RenderCaptureTarget, RenderEngine, RenderFrameCapture},
      theme::{PaletteColor, ShadowStyle},
    },
    components::{Column, Rect, Row, Stack},
    layout::{Constraints, Size, render_list::RenderList},
    node::{BoxShadow, Element, border::Border},
  };
  use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, Win32WindowHandle, WindowHandle,
  };
  use windows::{
    Win32::{
      Foundation::{HWND, LPARAM, LRESULT, WPARAM},
      UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW, WS_POPUP,
      },
    },
    core::w,
  };

  const SIZE: u32 = 320;

  extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
  }
  struct Surface(HWND);
  impl HasWindowHandle for Surface {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
      let raw = Win32WindowHandle::new(NonZeroIsize::new(self.0.0 as isize).unwrap());
      Ok(unsafe { WindowHandle::borrow_raw(raw.into()) })
    }
  }
  impl HasDisplayHandle for Surface {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
      Ok(DisplayHandle::windows())
    }
  }
  impl Drop for Surface {
    fn drop(&mut self) {
      unsafe {
        let _ = DestroyWindow(self.0);
      }
    }
  }
  struct Capture {
    backend: Box<dyn RenderEngine>,
    target: RenderCaptureTarget,
  }
  impl RenderEngine for Capture {
    fn resize(&mut self, w: u32, h: u32) {
      self.backend.resize(w, h);
    }
    fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>) -> bool {
      let request = RenderFrameCapture {
        x: 0,
        y: 0,
        width: SIZE,
        height: SIZE,
        target: self.target.clone(),
        window_clip: None,
      };
      self.backend.render_with_capture(list, window, display, Some(request))
    }
  }

  fn scene() -> Element {
    let card = |shadow: ShadowStyle| {
      Rect::new(80.0, 56.0)
        .background(PaletteColor::SurfaceRaised)
        .rounded(10.0)
        .box_shadow(shadow)
    };
    Stack::new()
      .size(SIZE as f32, SIZE as f32)
      .background("#eef0f3")
      .child(
        Column::new()
          .padding(24.0)
          .spacing(36.0)
          .overflow_visible()
          .child(
            Row::new()
              .spacing(24.0)
              .overflow_visible()
              .child(card(ShadowStyle::Sm))
              .child(card(ShadowStyle::Md))
              .child(card(ShadowStyle::extra("popover"))),
          )
          .child(
            Row::new()
              .spacing(24.0)
              .overflow_visible()
              // An inset well with a border.
              .child(
                Rect::new(96.0, 56.0)
                  .background("#ffffff")
                  .rounded(8.0)
                  .border(Border::inside(1.0, "#c8ccd4"))
                  .box_shadow(BoxShadow::new(0.0, 2.0, 6.0, "#0000004d").inset()),
              )
              // A hard, spread, offset shadow at half opacity.
              .child(
                Rect::new(64.0, 56.0)
                  .background(PaletteColor::Accent)
                  .opacity(0.5)
                  .box_shadow(BoxShadow::new(6.0, 6.0, 0.0, "#1f2937").spread(2.0)),
              ),
          )
          // A container that clips its child's shadow.
          .child(
            Stack::new()
              .size(200.0, 72.0)
              .rounded(12.0)
              .clip()
              .background("#dfe3ea")
              .child(
                Rect::new(120.0, 40.0)
                  .background("#ffffff")
                  .rounded(6.0)
                  .box_shadow(BoxShadow::new(0.0, 12.0, 32.0, "#000000a0").spread(4.0)),
              ),
          ),
      )
      .into()
  }

  let surface = Surface(unsafe {
    let class_name = w!("lurq_box_shadow_capture_check");
    assert_ne!(
      RegisterClassW(&WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        lpszClassName: class_name,
        ..Default::default()
      }),
      0
    );
    CreateWindowExW(
      WINDOW_EX_STYLE::default(),
      class_name,
      w!("Box shadow capture check"),
      WS_POPUP,
      0,
      0,
      SIZE as i32,
      SIZE as i32,
      None,
      None,
      None,
      None,
    )
    .unwrap()
  });

  let capture = |backend: &'static str| -> CapturedFrame {
    let (sender, receiver) = mpsc::channel();
    let target = RenderCaptureTarget::Bytes(Arc::new(move |frame| {
      let _ = sender.send(frame);
    }));
    let mut tree = Tree::new();
    tree.set_render_engine_factory(move || {
      let backend: Box<dyn RenderEngine> = match backend {
        "wgpu" => Box::new(lurq::app::wgpu_render::WgpuRenderEngine::new()),
        _ => Box::new(lurq::app::dx12_render::Dx12RenderEngine::new()),
      };
      Box::new(Capture {
        backend,
        target: target.clone(),
      })
    });
    tree.resize(SIZE, SIZE);
    tree.set_layout_constraints_override(Some(Constraints::tight(Size::new(SIZE as f32, SIZE as f32))));
    let mut app = App::new();
    app.theme().set_shadow_style(
      ShadowStyle::extra("popover"),
      vec![
        BoxShadow::new(0.0, 12.0, 24.0, "#0f172a40").spread(-4.0),
        BoxShadow::new(0.0, 0.0, 0.0, "#0f172a26").spread(1.0),
      ],
    );
    tree.set_root(scene());
    assert!(tree.pass(&mut app, &surface).rendered, "{backend}: no frame");
    receiver
      .recv_timeout(Duration::from_secs(15))
      .expect("capture")
      .expect("capture ok")
  };

  let wgpu = capture("wgpu");
  let dx12 = capture("dx12");
  for (name, frame) in [("wgpu", &wgpu), ("dx12", &dx12)] {
    let png = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone()).unwrap();
    let output = std::path::PathBuf::from("target").join(format!("box-shadow-{name}.png"));
    png.save(&output).unwrap();
    println!("{name}: {}", output.display());
  }

  let at = |frame: &CapturedFrame, x: u32, y: u32| {
    let i = ((y * frame.width + x) * 4) as usize;
    [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
  };
  for frame in [&wgpu, &dx12] {
    // Below the `md` card, inside its shadow: darker than the page.
    assert!(at(frame, 168, 82)[0] < 0xee, "md shadow {:?}", at(frame, 168, 82));
    // Well above the cards: the page colour, untouched by any shadow.
    assert_eq!(at(frame, 160, 4), [0xee, 0xf0, 0xf3]);
    // The clipped shadow does not leave its rounded container (y 208..280).
    assert_eq!(at(frame, 100, 306), [0xee, 0xf0, 0xf3], "shadow leaked out of its clip");
  }

  let mut worst = 0u8;
  let mut over_one = 0usize;
  for (a, b) in wgpu.rgba.chunks_exact(4).zip(dx12.rgba.chunks_exact(4)) {
    let diff = a.iter().zip(b).take(3).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    worst = worst.max(diff);
    over_one += usize::from(diff > 1);
  }
  println!("wgpu vs dx12: worst channel difference {worst}, {over_one} pixels differ by more than 1");
  assert!(worst <= 2, "backends disagree by {worst}");
  println!("box shadow capture check passed");
}

#[cfg(not(windows))]
fn main() {
  eprintln!("This capture harness uses a Win32 surface.");
}
