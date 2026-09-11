use super::*;

fn rect(x: f32) -> Path2D {
  let mut path = Path2D::new();
  path.rect(x, 0., 10., 10.);
  path
}
fn canvas() -> CanvasHandle {
  let canvas = CanvasHandle::new();
  canvas.inner.lock().attached = true;
  canvas
}
fn prepare(canvas: &CanvasHandle, cache: &mut MeshCache) -> Prepared {
  let batch = canvas.take_batch().unwrap();
  let prepared = Prepared::new(&batch.commands, cache).unwrap();
  batch.submit();
  prepared
}
fn area(vertices: &[Vertex]) -> f32 {
  vertices
    .chunks_exact(3)
    .map(|v| {
      let [a, b, c] = [v[0].position, v[1].position, v[2].position];
      ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
    })
    .sum()
}

#[test]
fn camera_color_rule_and_edits_use_the_right_mesh() {
  let canvas = canvas();
  let d = canvas.context_2d();
  let mut cache = MeshCache::default();
  let mut path = rect(0.);
  path.rect(2., 2., 6., 6.);
  for i in 0..20 {
    d.clear();
    d.set_transform(Transform2D::translate(i as f32, 3.).then(&Transform2D::scale(2., -3.)));
    d.set_fill_style(if i % 2 == 0 { "#ff000080" } else { "#00ff00" });
    d.fill_path(&path, FillRule::EvenOdd);
    let p = prepare(&canvas, &mut cache);
    assert!((area(&p.vertices) - 64. * 6.).abs() < 0.01);
    assert_eq!(p.draws[0].bounds, [i as f32, -27., i as f32 + 20., 3.]);
    assert_eq!(p.vertices[0].color[1], if i % 2 == 0 { 0. } else { 1. });
  }
  assert_eq!(
    cache.misses, 1,
    "polygon pans/zooms/reflections and color changes reuse geometry"
  );
  d.fill_path(&path, FillRule::NonZero);
  assert!((area(&prepare(&canvas, &mut cache).vertices) - 100. * 6.).abs() < 0.01);
  assert_eq!(cache.misses, 2, "fill rule affects the triangles");
  let old = path.clone();
  d.fill_path(&path, FillRule::NonZero);
  path.rect(20., 0., 10., 10.);
  assert!(
    (area(&prepare(&canvas, &mut cache).vertices) - 600.).abs() < 0.01,
    "queued path is immutable"
  );
  d.fill_path(&path, FillRule::NonZero);
  assert!((area(&prepare(&canvas, &mut cache).vertices) - 1200.).abs() < 0.01);
  d.fill_path(&old, FillRule::NonZero);
  prepare(&canvas, &mut cache);
  assert_eq!(cache.misses, 3, "editing a clone does not invalidate its sibling");
  let mut rebuilt = rect(0.);
  rebuilt.rect(2., 2., 6., 6.);
  d.fill_path(&rebuilt, FillRule::EvenOdd);
  prepare(&canvas, &mut cache);
  assert_eq!(cache.misses, 3, "identical rebuilt geometry hits by content");
}

#[test]
fn curves_follow_scale_buckets_including_dpi_skew_and_rotation() {
  let mut path = Path2D::new();
  path
    .arc(0., 0., 20., 0., std::f32::consts::TAU, ArcDirection::Clockwise)
    .unwrap();
  let path = path.geometry().unwrap();
  let mut cache = MeshCache::default();
  for scale in [1.1, 1.5, 1.9, 1.2] {
    let m = Transform2D::translate(73., -8.).then(&Transform2D::scale_uniform(scale));
    cache
      .append(&path, m, FillRule::NonZero, [1.; 4], &mut Vec::new())
      .unwrap();
    assert_eq!(tolerance(&path, m), 0.05);
  }
  assert_eq!(cache.misses, 1);
  let m = Transform2D::scale_uniform(2.).then(&Transform2D::scale_uniform(1.1));
  cache
    .append(&path, m, FillRule::NonZero, [1.; 4], &mut Vec::new())
    .unwrap();
  assert_eq!(cache.misses, 2, "DPI participates in flattening scale");
  assert_eq!(tolerance(&path, m), 0.025);
  for m in [
    Transform2D::rotate(0.7).then(&Transform2D::scale(-3., 0.5)),
    Transform2D::skew(1.2, 0.3),
  ] {
    let tolerance = tolerance(&path, m);
    for i in 0..360 {
      let (y, x) = (i as f32 * std::f32::consts::PI / 180.).sin_cos();
      let (x, y) = m.transform_point(x, y);
      assert!(tolerance * x.hypot(y) <= 0.10001);
    }
  }
}

#[test]
fn clip_transform_is_frozen_and_stroke_style_changes_invalidate() {
  let canvas = canvas();
  canvas.inner.lock().metrics.scale_factor = 2.;
  let d = canvas.context_2d();
  let mut cache = MeshCache::default();
  let path = rect(0.);
  d.set_transform(Transform2D::translate(5., 7.));
  d.clip_path(&path, FillRule::NonZero);
  d.set_transform(Transform2D::scale_uniform(3.));
  d.set_line_width(2.);
  d.stroke_path(&path);
  let p = prepare(&canvas, &mut cache);
  let clip = p.draws[0].clips[0].clone();
  let clip_vertices = &p.vertices[clip.start as usize..clip.end as usize];
  assert!((area(clip_vertices) - 400.).abs() < 0.01);
  assert!(
    clip_vertices
      .iter()
      .all(|v| v.position[0] >= 10. && v.position[0] <= 30. && v.position[1] >= 14. && v.position[1] <= 34.)
  );
  assert_eq!(p.draws[0].bounds, [-6., -6., 66., 66.]);
  let misses = cache.misses;
  d.translate(4., 0.);
  d.stroke_path(&path);
  prepare(&canvas, &mut cache);
  assert_eq!(cache.misses, misses, "unchanged outline and clip reuse triangles");
  d.set_line_width(4.);
  d.stroke_path(&path);
  prepare(&canvas, &mut cache);
  assert_eq!(cache.misses, misses + 1);
  d.set_line_dash(&[2., 2.]);
  d.stroke_path(&path);
  prepare(&canvas, &mut cache);
  assert_eq!(cache.misses, misses + 2);
  d.set_line_dash_offset(1.);
  d.stroke_path(&path);
  prepare(&canvas, &mut cache);
  assert_eq!(cache.misses, misses + 3);
}

#[test]
fn cache_eviction_bounds_payload_metadata_and_oversized_entries() {
  let key = |i| Key {
    path: rect(i as f32).geometry().unwrap(),
    rule: FillRule::NonZero,
    tolerance: 0.1_f32.to_bits(),
  };
  let mut cache = MeshCache::default();
  let positions: Arc<[[f32; 2]]> = vec![[0.; 2]; MAX_CACHE_BYTES / 8 / 4].into();
  for i in 0..10 {
    cache.insert(key(i), positions.clone());
    assert!(cache.bytes <= MAX_CACHE_BYTES);
    assert_eq!(cache.order.len(), cache.entries.len());
  }
  assert_eq!(cache.entries.len(), 3);
  assert!(!cache.entries.contains_key(&key(0)));
  let oversized: Arc<[[f32; 2]]> = vec![[0.; 2]; MAX_CACHE_BYTES / 8].into();
  cache.insert(key(50), oversized);
  assert_eq!(cache.entries.len(), 3, "oversized entries bypass retention");
  let mut cache = MeshCache::default();
  for i in 0..MAX_CACHE_ENTRIES + 10 {
    cache.insert(key(i), Arc::from([]));
  }
  assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);
  assert_eq!(cache.order.len(), MAX_CACHE_ENTRIES);
  assert!(cache.bytes <= MAX_CACHE_BYTES);
  assert!(!cache.entries.contains_key(&key(0)));
}

#[test]
fn cached_meshes_preserve_vertex_limit_and_reject_nonfinite_transforms() {
  let mut cache = MeshCache::default();
  let path = rect(0.).geometry().unwrap();
  let mut output = Vec::new();
  cache
    .append(&path, Transform2D::IDENTITY, FillRule::NonZero, [1.; 4], &mut output)
    .unwrap();
  output.resize(MAX_VERTICES - 3, output[0]);
  assert_eq!(
    cache.append(&path, Transform2D::IDENTITY, FillRule::NonZero, [1.; 4], &mut output),
    Err(CanvasError::StateLimit)
  );
  assert_eq!(cache.misses, 1, "the limit applies on cache hits");
  output.clear();
  cache
    .append(
      &path,
      Transform2D::scale_uniform(f32::MAX),
      FillRule::NonZero,
      [1.; 4],
      &mut output,
    )
    .unwrap();
  assert!(output.is_empty());
}
