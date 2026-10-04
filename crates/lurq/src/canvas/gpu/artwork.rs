use super::*;
/// Six fixed vertices: no path tessellation, text shaping, or CPU image readback.
pub(crate) fn artwork_vertices(width: u32, height: u32, matrix: Transform2D) -> [Vertex; 6] {
  [[0., 0.], [1., 0.], [0., 1.], [0., 1.], [1., 0.], [1., 1.]].map(|uv| {
    let (x, y) = matrix.transform_point(uv[0] * width as f32, uv[1] * height as f32);
    Vertex {
      position: [x, y],
      uv,
      color: [1.; 4],
    }
  })
}
