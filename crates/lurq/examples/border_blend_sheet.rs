//! A sheet of borders on some sides of boxes with square, rounded and pill corners, in a light and a dark
//! appearance, rendered through layout and the wgpu backend on a hidden Win32 surface; the dx12 backend
//! renders it too and the two are compared.
//! cargo run -p lurq --example border_blend_sheet --features screenshot,wgpu,dx12 -- <name>
//! Writes target/border-blend-<name>.png, and parts of it at 4x to target/border-blend-<name>-zoom*.png;
//! `-- <name> --zoom-only` cuts the parts out of a sheet written before.
//!
//! Columns: radius 0, 6, 8, 16 and a pill, each with widths 1, 2 and 3. Rows: a bottom border; bottom and
//! left in one color; bottom and left in two; left, bottom and right; a dashed bottom; a bottom outside the
//! box; a bottom centered on its edge; every side.
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
    },
    components::{Column, Rect, Row, Stack},
    layout::{Constraints, Size, render_list::RenderList},
    node::{Element, border::Border},
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

  const BOX_W: f32 = 64.0;
  const BOX_H: f32 = 40.0;
  const GAP: f32 = 10.0;
  const GROUP_GAP: f32 = 28.0;
  const ROW_GAP: f32 = 14.0;
  const PAD: f32 = 20.0;
  const RADII: [f32; 5] = [0.0, 6.0, 8.0, 16.0, 999.0];
  const WIDTHS: [f32; 3] = [1.0, 2.0, 3.0];
  const ROWS: usize = 8;
  const GROUP_W: f32 = 3.0 * BOX_W + 2.0 * GAP;
  const PANEL_W: f32 = 2.0 * PAD + 5.0 * GROUP_W + 4.0 * GROUP_GAP;
  const PANEL_H: f32 = 2.0 * PAD + ROWS as f32 * BOX_H + (ROWS - 1) as f32 * ROW_GAP;
  const WIDTH: u32 = PANEL_W as u32;
  const HEIGHT: u32 = 2 * PANEL_H as u32;

  struct Palette {
    page: &'static str,
    fill: &'static str,
    line: &'static str,
    accent: &'static str,
    second: &'static str,
  }
  const LIGHT: Palette = Palette {
    page: "#ffffff",
    fill: "#f3f4f6",
    line: "#9aa1ad",
    accent: "#2563eb",
    second: "#ea580c",
  };
  const DARK: Palette = Palette {
    page: "#111317",
    fill: "#1c1f26",
    line: "#5b6270",
    accent: "#60a5fa",
    second: "#fb923c",
  };

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
        width: WIDTH,
        height: HEIGHT,
        target: self.target.clone(),
        window_clip: None,
      };
      self.backend.render_with_capture(list, window, display, Some(request))
    }
  }

  fn sample(palette: &Palette, row: usize, radius: f32, width: f32) -> Element {
    let rect = Rect::new(BOX_W, BOX_H).background(palette.fill).rounded(radius);
    let line = |color: &'static str| Border::inside(width, color);
    match row {
      0 => rect.border_bottom(line(palette.accent)),
      1 => rect.border_bottom(line(palette.line)).border_left(line(palette.line)),
      2 => rect
        .border_bottom(line(palette.accent))
        .border_left(line(palette.second)),
      3 => rect
        .border_left(line(palette.line))
        .border_bottom(line(palette.line))
        .border_right(line(palette.line)),
      4 => rect.border_bottom(line(palette.accent).dashed()),
      5 => rect.border_bottom(Border::outside(width, palette.accent)),
      6 => rect.border_bottom(Border::center(width, palette.accent)),
      _ => rect.border(line(palette.line)),
    }
    .into()
  }

  fn panel(palette: &Palette) -> Element {
    let mut rows = Column::new().padding(PAD).spacing(ROW_GAP).overflow_visible();
    for row in 0..ROWS {
      let mut groups = Row::new().spacing(GROUP_GAP).overflow_visible();
      for radius in RADII {
        let mut group = Row::new().spacing(GAP).overflow_visible();
        for width in WIDTHS {
          group = group.child(sample(palette, row, radius, width));
        }
        groups = groups.child(group);
      }
      rows = rows.child(groups);
    }
    Stack::new()
      .size(PANEL_W, PANEL_H)
      .background(palette.page)
      .child(rows)
      .into()
  }

  fn scene() -> Element {
    Column::new()
      .size(PANEL_W, 2.0 * PANEL_H)
      .child(panel(&LIGHT))
      .child(panel(&DARK))
      .into()
  }

  /// Parts of the sheet in both appearances at 4x: radius 8 and 16 in the first four rows, and radius 16 and
  /// the pill in the dashed, outside and centered rows.
  fn save_zooms(sheet: &image::RgbaImage, name: &str) {
    for (suffix, groups, rows) in [("zoom", 2..4, 0..4), ("zoom-placements", 3..5, 4..7)] {
      let x0 = (PAD + groups.start as f32 * (GROUP_W + GROUP_GAP) - 6.0) as u32;
      let x1 = (PAD + groups.end as f32 * (GROUP_W + GROUP_GAP) - GROUP_GAP + 6.0) as u32;
      let y0 = (PAD + rows.start as f32 * (BOX_H + ROW_GAP) - 6.0) as u32;
      let count = rows.len() as f32;
      let (crop_w, crop_h) = (x1 - x0, (count * BOX_H + (count - 1.0) * ROW_GAP + 12.0) as u32);
      let mut zoom = image::RgbaImage::new(crop_w * 4, crop_h * 8);
      for (panel, top) in [y0, y0 + PANEL_H as u32].into_iter().enumerate() {
        let part = image::imageops::crop_imm(sheet, x0, top, crop_w, crop_h).to_image();
        let part = image::imageops::resize(&part, crop_w * 4, crop_h * 4, image::imageops::FilterType::Nearest);
        image::imageops::replace(&mut zoom, &part, 0, (panel as u32 * crop_h * 4) as i64);
      }
      let output = std::path::PathBuf::from("target").join(format!("border-blend-{name}-{suffix}.png"));
      zoom.save(&output).unwrap();
      println!("{}", output.display());
    }
  }

  let name = std::env::args().nth(1).unwrap_or_else(|| "sheet".to_owned());
  let output = std::path::PathBuf::from("target").join(format!("border-blend-{name}.png"));
  // `-- <name> --zoom-only` cuts the parts out of a sheet saved before.
  if std::env::args().nth(2).as_deref() == Some("--zoom-only") {
    save_zooms(&image::open(&output).unwrap().to_rgba8(), &name);
    return;
  }
  let surface = Surface(unsafe {
    let class_name = w!("lurq_border_blend_sheet");
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
      w!("Border blend sheet"),
      WS_POPUP,
      0,
      0,
      WIDTH as i32,
      HEIGHT as i32,
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
    tree.resize(WIDTH, HEIGHT);
    tree.set_layout_constraints_override(Some(Constraints::tight(Size::new(WIDTH as f32, HEIGHT as f32))));
    let mut app = App::new();
    tree.set_root(scene());
    assert!(tree.pass(&mut app, &surface).rendered, "{backend}: no frame");
    receiver
      .recv_timeout(Duration::from_secs(15))
      .expect("capture")
      .expect("capture ok")
  };

  let wgpu = capture("wgpu");
  let dx12 = capture("dx12");
  let sheet = image::RgbaImage::from_raw(wgpu.width, wgpu.height, wgpu.rgba.clone()).unwrap();
  sheet.save(&output).unwrap();
  println!("{}", output.display());
  save_zooms(&sheet, &name);

  let mut worst = 0u8;
  let mut over_two = 0usize;
  for (a, b) in wgpu.rgba.chunks_exact(4).zip(dx12.rgba.chunks_exact(4)) {
    let diff = a.iter().zip(b).take(3).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    worst = worst.max(diff);
    over_two += usize::from(diff > 2);
  }
  println!("wgpu vs dx12: worst channel difference {worst}, {over_two} pixels differ by more than 2");
}

#[cfg(not(windows))]
fn main() {
  eprintln!("This capture harness uses a Win32 surface.");
}
