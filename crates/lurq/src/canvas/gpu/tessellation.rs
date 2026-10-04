use super::*;
#[derive(Default)]
pub(crate) struct Prepared {
  pub vertices: Vec<Vertex>,
  pub steps: Vec<Step>,
  pub clear: bool,
}
impl Prepared {
  pub fn new(commands: &[Command], cache: &mut MeshCache) -> Result<Self, CanvasError> {
    let mut result = Self::default();
    let mut clips = HashMap::new();
    let mut open: Vec<(usize, [f32; 4])> = Vec::new();
    for command in commands {
      match command {
        Command::Clear => {
          result = Self {
            clear: true,
            ..Self::default()
          };
          clips.clear();
          open.clear();
          continue;
        }
        Command::BeginLayer { alpha, blend } => {
          open.push((result.steps.len(), NO_BOUNDS));
          result.steps.push(Step::Begin {
            alpha: *alpha,
            blend: *blend,
            bounds: NO_BOUNDS,
            end: 0,
          });
          continue;
        }
        Command::EndLayer => {
          close_layer(&mut result.steps, &mut open);
          continue;
        }
        _ => {}
      }
      let (clip, scale) = match command {
        Command::Path { clip, scale, .. } | Command::Image { clip, scale, .. } => (clip, *scale),
        _ => continue,
      };
      let mut clip_ranges = Vec::new();
      let mut chain = Vec::new();
      let mut clip = clip.as_ref();
      while let Some(c) = clip {
        chain.push(c);
        clip = c.previous.as_ref();
      }
      for clip in chain.into_iter().rev() {
        let key = (Arc::as_ptr(clip) as usize, scale.to_bits());
        let range = if let Some(range) = clips.get(&key) {
          Range::<u32>::clone(range)
        } else {
          let start = result.vertices.len() as u32;
          if let Some(path) = &clip.path {
            cache.append(
              path,
              Transform2D::scale_uniform(scale).then(&clip.matrix),
              clip.rule,
              [0.; 4],
              None,
              &mut result.vertices,
            )?;
          }
          let range = start..result.vertices.len() as u32;
          clips.insert(key, range.clone());
          range
        };
        clip_ranges.push(range);
      }
      let start = result.vertices.len() as u32;
      let (asset, smooth, erase, kind) = match command {
        Command::Path {
          path,
          matrix,
          rule,
          color,
          gradient,
          erase,
          ..
        } => {
          cache.append(
            path,
            *matrix,
            *rule,
            *color,
            gradient.as_ref().map(|g| g.frame),
            &mut result.vertices,
          )?;
          match gradient {
            Some(gradient) => (
              Some(gradient.ramp.clone()),
              true,
              *erase,
              DrawKind::Gradient(gradient.kind),
            ),
            None => (None, false, *erase, DrawKind::Solid),
          }
        }
        Command::Image {
          asset,
          matrix,
          source: [x, y, w, h],
          alpha,
          smooth,
          ..
        } => {
          let corners = [[*x, *y], [x + w, *y], [x + w, y + h], [*x, y + h]];
          for index in [0, 1, 2, 0, 2, 3] {
            let [x, y] = corners[index];
            let (px, py) = matrix.transform_point(x, y);
            result.vertices.push(Vertex {
              position: [px, py],
              uv: [x / asset.width as f32, y / asset.height as f32],
              color: [*alpha; 4],
            });
          }
          (Some(asset.clone()), *smooth, false, DrawKind::Image)
        }
        _ => unreachable!(),
      };
      let end = result.vertices.len() as u32;
      if end as usize > MAX_VERTICES {
        return Err(CanvasError::StateLimit);
      }
      let mut bounds = NO_BOUNDS;
      for v in &result.vertices[start as usize..end as usize] {
        bounds[0] = bounds[0].min(v.position[0]);
        bounds[1] = bounds[1].min(v.position[1]);
        bounds[2] = bounds[2].max(v.position[0]);
        bounds[3] = bounds[3].max(v.position[1]);
      }
      for layer in &mut open {
        cover(&mut layer.1, bounds);
      }
      result.steps.push(Step::Draw(Draw {
        vertices: start..end,
        clips: clip_ranges,
        bounds,
        asset,
        smooth,
        erase,
        kind,
      }));
    }
    // A batch is taken whole, so an unmatched `BeginLayer` cannot normally reach
    // a backend. Closing one here keeps a backend's own layer stack balanced
    // rather than making every backend defend itself.
    while !open.is_empty() {
      close_layer(&mut result.steps, &mut open);
    }
    Ok(result)
  }
  pub fn draws(&self) -> impl Iterator<Item = &Draw> {
    self.steps.iter().filter_map(Step::draw)
  }
  pub fn tiles(&self, width: u32, height: u32) -> Vec<[u32; 4]> {
    let mut result = Vec::new();
    for y in (0..height).step_by(TILE as usize) {
      for x in (0..width).step_by(TILE as usize) {
        let tile = [x, y, TILE.min(width - x), TILE.min(height - y)];
        if self.draws().any(|d| d.intersects(tile)) {
          result.push(tile);
        }
      }
    }
    result
  }
}
fn close_layer(steps: &mut Vec<Step>, open: &mut Vec<(usize, [f32; 4])>) {
  let Some((begin, bounds)) = open.pop() else {
    return;
  };
  let end = steps.len();
  steps.push(Step::End);
  if let Some(Step::Begin {
    bounds: slot,
    end: index,
    ..
  }) = steps.get_mut(begin)
  {
    *slot = bounds;
    *index = end;
  }
  if let Some(parent) = open.last_mut() {
    cover(&mut parent.1, bounds);
  }
}
impl Draw {
  pub fn intersects(&self, tile: [u32; 4]) -> bool {
    touches(self.bounds, tile)
  }
}
/// The deepest isolation a batch opens, which is how many tile-sized copies a
/// backend has to hold while it draws it.
pub(crate) fn layer_depth(prepared: &Prepared) -> usize {
  let (mut depth, mut deepest) = (0usize, 0usize);
  for step in &prepared.steps {
    match step {
      Step::Begin { .. } => {
        depth += 1;
        deepest = deepest.max(depth);
      }
      Step::End => depth = depth.saturating_sub(1),
      Step::Draw(_) => {}
    }
  }
  deepest
}
/// Whether work bounded by `bounds` can change anything in `tile`.
pub(crate) fn touches(bounds: [f32; 4], [x, y, w, h]: [u32; 4]) -> bool {
  bounds[0] < (x + w) as f32 && bounds[1] < (y + h) as f32 && bounds[2] > x as f32 && bounds[3] > y as f32
}
fn mesh(
  path: &Path,
  rule: FillRule,
  tolerance: f32,
  color: [f32; 4],
  output: &mut Vec<Vertex>,
) -> Result<(), CanvasError> {
  let mut builder = LyonPath::builder();
  let mut open = false;
  for segment in path.segments() {
    use tiny_skia::PathSegment::*;
    match segment {
      MoveTo(p) => {
        if open {
          builder.end(true);
        }
        builder.begin(point(p.x, p.y));
        open = true;
      }
      LineTo(p) => {
        builder.line_to(point(p.x, p.y));
      }
      QuadTo(a, b) => {
        builder.quadratic_bezier_to(point(a.x, a.y), point(b.x, b.y));
      }
      CubicTo(a, b, c) => {
        builder.cubic_bezier_to(point(a.x, a.y), point(b.x, b.y), point(c.x, c.y));
      }
      Close => {
        if open {
          builder.end(true);
          open = false;
        }
      }
    }
  }
  if open {
    builder.end(true);
  }
  let path = builder.build();
  let options = FillOptions::default()
    .with_tolerance(tolerance)
    .with_fill_rule(match rule {
      FillRule::NonZero => lyon::path::FillRule::NonZero,
      FillRule::EvenOdd => lyon::path::FillRule::EvenOdd,
    });
  // The custom builder limits expansion while tessellating, including malicious
  // self-intersecting paths whose triangulation is much larger than their input.
  let mut geometry = LimitedGeometry {
    vertices: Vec::new(),
    output,
    color,
    overflow: false,
  };
  FillTessellator::new()
    .tessellate_path(&path, &options, &mut geometry)
    .map_err(|_| CanvasError::StateLimit)?;
  if geometry.overflow {
    return Err(CanvasError::StateLimit);
  }
  Ok(())
}
struct LimitedGeometry<'a> {
  vertices: Vec<[f32; 2]>,
  output: &'a mut Vec<Vertex>,
  color: [f32; 4],
  overflow: bool,
}
impl GeometryBuilder for LimitedGeometry<'_> {
  fn begin_geometry(&mut self) {}
  fn end_geometry(&mut self) {}
  fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
    if self.output.len() + 3 <= MAX_VERTICES {
      for i in [a, b, c] {
        self.output.push(Vertex {
          position: self.vertices[i.to_usize()],
          uv: [0.; 2],
          color: self.color,
        });
      }
    } else {
      self.overflow = true;
    }
  }
  fn abort_geometry(&mut self) {}
}
impl FillGeometryBuilder for LimitedGeometry<'_> {
  fn add_fill_vertex(&mut self, vertex: FillVertex<'_>) -> Result<VertexId, GeometryBuilderError> {
    if self.vertices.len() >= MAX_VERTICES || self.output.len() + 3 > MAX_VERTICES {
      return Err(GeometryBuilderError::TooManyVertices);
    }
    let id = VertexId(self.vertices.len() as u32);
    let p = vertex.position();
    self.vertices.push([p.x, p.y]);
    Ok(id)
  }
}
