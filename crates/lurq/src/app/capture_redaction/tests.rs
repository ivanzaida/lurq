use super::*;

fn glyph(x: f32, y: f32, width: f32, height: f32) -> GlyphCmd {
  GlyphCmd {
    order: 0,
    x,
    y,
    width,
    height,
    color: [1.0; 4],
    atlas_min: [0.0; 2],
    atlas_max: [0.0; 2],
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0; 2],
    sharpness: 1.0,
    color_glyph: false,
    shadow_sigma: 0.0,
    clip: ClipRect::default(),
  }
}

fn area(x0: u32, y0: u32, x1: u32, y1: u32) -> Option<RedactedArea> {
  Some(RedactedArea { x0, y0, x1, y1 })
}

#[test]
fn a_run_is_covered_with_its_edges_shadow_and_transform() {
  // Each glyph's rect, grown by the anti-aliased edge reach, joined.
  let run = [glyph(10.0, 20.0, 8.0, 12.0), glyph(30.5, 18.0, 6.0, 10.0)];
  assert_eq!(RedactedArea::of_glyph_run(&run), area(8, 16, 39, 34));

  // A text-shadow instance reaches 2 sigma plus a texel past its rect.
  let mut shadow = glyph(10.0, 20.0, 8.0, 12.0);
  shadow.shadow_sigma = 3.0;
  assert_eq!(RedactedArea::of_glyph_run(&[shadow]), area(1, 11, 27, 41));

  // A quarter turn about the glyph's centre swaps its extents.
  let mut turned = glyph(10.0, 20.0, 8.0, 12.0);
  turned.transform = [0.0, 1.0, -1.0, 0.0];
  turned.transform_origin = [4.0, 6.0];
  assert_eq!(RedactedArea::of_glyph_run(&[turned]), area(6, 20, 22, 32));
}

#[test]
fn a_run_is_limited_to_its_clip() {
  let clip = ClipRect {
    x: 0.0,
    y: 0.0,
    width: 15.0,
    height: 100.0,
    active: true,
    border_radius: None,
  };
  let mut clipped = glyph(10.0, 20.0, 8.0, 12.0);
  clipped.clip = clip;
  assert_eq!(RedactedArea::of_glyph_run(&[clipped.clone()]), area(8, 18, 15, 34));

  let mut hidden = glyph(40.0, 20.0, 8.0, 12.0);
  hidden.clip = clip;
  assert_eq!(RedactedArea::of_glyph_run(&[hidden]), None);
  assert_eq!(RedactedArea::of_glyph_run(&[]), None);
}

#[test]
fn redaction_paints_the_area_where_it_falls_in_a_cropped_capture() {
  // A 4x4 capture of window pixels (10, 10)..(14, 14).
  let mut pixels = [0_u8; 4 * 4 * 4];
  let areas = [RedactedArea {
    x0: 11,
    y0: 11,
    x1: 13,
    y1: 20,
  }];
  redact_pixels(&mut pixels, 4, 4, (10, 10), &areas);
  for y in 0..4 {
    for x in 0..4 {
      let index = (y * 4 + x) * 4;
      let expected = if (1..3).contains(&x) && y >= 1 {
        REDACTION_FILL
      } else {
        [0; 4]
      };
      assert_eq!(pixels[index..index + 4], expected, "pixel {x},{y}");
    }
  }
}
