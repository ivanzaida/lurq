//! cargo run -p lurq --example canvas --features canvas,winit,wgpu
#[path = "support/canvas_scene.rs"]
mod canvas_scene;

use lurq::app::{App, Tree, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow};

fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  tree.mount_root::<canvas_scene::CanvasScene>(&mut app, ());
  WinitWindow::new(app, tree)
    .with_size(256, 256)
    .with_title("Canvas: click to draw through the existing ref")
    .run();
}
