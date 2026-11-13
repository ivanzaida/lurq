cbuffer Globals : register(b0)
{
  float4 viewport;
  float4 clip_rect;
  float4 clip_radii_h;
  float4 clip_radii_v;
  float4 clip_active;      // x: cut by the innermost rounded clip; y: by the one outside it too
  float4 outer_clip_rect;  // the rounded clip outside the innermost one: x, y, w, h
  float4 outer_clip_radii; // and its radii: tl, tr, br, bl
};

// Gradient stop storage. Layout per gradient (each element is a float4):
//   [count, kind, flags, from_angle]
//   [dir.x, dir.y, center.x, center.y]
//   then per stop: [r, g, b, a], [position, _, _, _]
// kind: 0 = linear, 1 = radial (flags bit0 = ellipse), 2 = conic.
StructuredBuffer<float4> gradients : register(t0);

struct VsIn
{
  float2 corner : TEXCOORD0;
  float2 pos : TEXCOORD1;
  float2 size : TEXCOORD2;
  float4 color : TEXCOORD3;
  float4 radii_h : TEXCOORD4;
  float4 radii_v : TEXCOORD5;
  float4 stroke : TEXCOORD6;
  float4 pattern : TEXCOORD7;
  float4 transform : TEXCOORD8;
  float2 xf_origin : TEXCOORD9;
  float shadow_sigma : TEXCOORD10;
  float gradient_offset : TEXCOORD11;
};

struct VsOut
{
  float4 position : SV_POSITION;
  float4 color : COLOR0;
  float2 local : TEXCOORD0;
  float2 half_size : TEXCOORD1;
  float4 radii_h : TEXCOORD2;
  float4 radii_v : TEXCOORD3;
  float4 stroke : TEXCOORD4;
  float gradient_offset : TEXCOORD5;
  float4 pattern : TEXCOORD6;
  float shadow_sigma : TEXCOORD7;
};

// Box shadows. The same formula as `blurred_rounded_rect_coverage` in
// `layout/box_shadow.rs` and `quad.wgsl`; keep the three in step. A shadow
// instance has no stroke and `pattern.x` = 1 (outer) or 2 (inset); `radii_h`
// are the element box's radii, `radii_v` the shadow shape's, `pattern.yz` the
// shape offset, `pattern.w` the spread and `shadow_sigma` the Gaussian's
// standard deviation.
static const int SHADOW_Y_SAMPLES = 8;
static const float SHADOW_EXTENT_SIGMAS = 3.0;
static const float SHADOW_MIN_SIGMA = 0.1;

VsOut vs_main(VsIn input)
{
  float aa_outset = 2.0;
  float max_stroke = max(max(input.stroke.x, input.stroke.y), max(input.stroke.z, input.stroke.w));
  if (max_stroke <= 0.0 && input.pattern.x > 0.5 && input.pattern.x < 1.5)
  {
    // An outer shadow paints beyond its element box.
    aa_outset += max(input.pattern.w, 0.0) + max(abs(input.pattern.y), abs(input.pattern.z))
      + SHADOW_EXTENT_SIGMAS * input.shadow_sigma;
  }
  if (max_stroke > 0.0)
  {
    // Round a corner whose other side has none, a stroke outside the box or
    // centred on its edge reaches past that side as past its own.
    aa_outset += max(input.pattern.w, 0.0) * max_stroke;
  }
  float2 local_px = input.corner * (input.size + float2(aa_outset * 2.0, aa_outset * 2.0))
    - float2(aa_outset, aa_outset);
  float2 centered = local_px - input.xf_origin;
  float2 transformed = float2(
    input.transform.x * centered.x + input.transform.z * centered.y,
    input.transform.y * centered.x + input.transform.w * centered.y
  );
  float2 px = input.pos + transformed + input.xf_origin;
  // `viewport.zw` is the target's origin in window pixels (non-zero when
  // painting into an opacity layer).
  float2 target_px = px - viewport.zw;
  float2 ndc = float2((target_px.x / viewport.x) * 2.0 - 1.0, 1.0 - (target_px.y / viewport.y) * 2.0);

  VsOut output;
  output.position = float4(ndc, 0.0, 1.0);
  output.color = input.color;
  output.local = local_px - input.size * 0.5;
  output.half_size = input.size * 0.5;
  output.radii_h = input.radii_h;
  output.radii_v = input.radii_v;
  output.stroke = input.stroke;
  output.gradient_offset = input.gradient_offset;
  output.pattern = input.pattern;
  output.shadow_sigma = input.shadow_sigma;
  return output;
}

float4 sample_gradient(int off, float2 local, float2 half_size)
{
  float4 h0 = gradients[off];
  float4 h1 = gradients[off + 1];
  int count = (int)h0.x;
  int kind = (int)h0.y;
  const float PI = 3.14159265359;

  float t;
  if (kind == 0)
  {
    float2 dir = h1.xy;
    float hl = abs(half_size.x * dir.x) + abs(half_size.y * dir.y);
    t = (dot(local, dir) + hl) / (2.0 * max(hl, 1e-5));
  }
  else if (kind == 1)
  {
    float2 center = (h1.zw * 2.0 - float2(1.0, 1.0)) * half_size;
    if (h0.z > 0.5)
    {
      float2 cn = h1.zw * 2.0 - float2(1.0, 1.0);
      float2 p = (local - center) / max(half_size, float2(1e-3, 1e-3));
      float radius = max(
        max(length(float2(-1.0, -1.0) - cn), length(float2(1.0, -1.0) - cn)),
        max(length(float2(-1.0, 1.0) - cn), length(float2(1.0, 1.0) - cn)));
      t = length(p) / max(radius, 1e-5);
    }
    else
    {
      float radius = max(
        max(length(float2(-half_size.x, -half_size.y) - center), length(float2(half_size.x, -half_size.y) - center)),
        max(length(float2(-half_size.x, half_size.y) - center), length(float2(half_size.x, half_size.y) - center)));
      t = length(local - center) / max(radius, 1e-5);
    }
  }
  else
  {
    float2 center = (h1.zw * 2.0 - float2(1.0, 1.0)) * half_size;
    float2 d = local - center;
    float ang = (atan2(d.x, -d.y) - h0.w) / (2.0 * PI);
    t = ang - floor(ang);
  }

  if (kind != 2)
  {
    t = saturate(t);
  }

  int stop_base = off + 2;
  int last = count - 1;
  float4 color = gradients[stop_base + 2 * last];
  for (int i = 0; i < last; i = i + 1)
  {
    float pb = gradients[stop_base + 2 * (i + 1) + 1].x;
    if (t <= pb)
    {
      float pa = gradients[stop_base + 2 * i + 1].x;
      float4 ca = gradients[stop_base + 2 * i];
      float4 cb = gradients[stop_base + 2 * (i + 1)];
      float span = max(pb - pa, 1e-5);
      color = lerp(ca, cb, saturate((t - pa) / span));
      break;
    }
  }
  return color;
}

float2 pick_radius(float2 p, float4 radii_h, float4 radii_v)
{
  if (p.y < 0.0)
  {
    return p.x < 0.0 ? float2(radii_h.x, radii_v.x) : float2(radii_h.y, radii_v.y);
  }
  return p.x < 0.0 ? float2(radii_h.w, radii_v.w) : float2(radii_h.z, radii_v.z);
}

float sd_rounded_box(float2 p, float2 half_size, float2 radius)
{
  float2 safe_radius = max(radius, float2(0.0, 0.0));
  float2 q = abs(p) - half_size + safe_radius;
  if (q.x > 0.0 && q.y > 0.0)
  {
    if (safe_radius.x <= 0.0 || safe_radius.y <= 0.0)
    {
      return max(q.x, q.y);
    }
    float2 pn = q / safe_radius;
    float len_pn = length(pn);
    if (len_pn <= 1e-6)
    {
      return -min(safe_radius.x, safe_radius.y);
    }
    float gradient = max(length(pn / safe_radius), 1e-6);
    return (len_pn - 1.0) * len_pn / gradient;
  }
  return max(q.x - safe_radius.x, q.y - safe_radius.y);
}

float aa_width(float dist)
{
  return max(fwidth(dist), 1.0);
}

// Ramp width for one of the four half-pixel subsamples: half a pixel, so a
// pixel fully inside an axis-aligned edge gets full coverage. Matches quad.wgsl.
float subsample_aa_width(float dist)
{
  return 0.5 * aa_width(dist);
}

// How much of the pixel at `frag_pos` (window pixels) the rounded clip at
// `rect` with these radii covers.
float rounded_clip_coverage(float2 frag_pos, float4 rect, float4 radii_h, float4 radii_v)
{
  float2 half_size = rect.zw * 0.5;
  float2 local = frag_pos - (rect.xy + half_size);
  float dist = sd_rounded_box(local, half_size, pick_radius(local, radii_h, radii_v));
  return saturate(0.5 - dist / aa_width(dist));
}

// How much of the pixel the rounded clips the draw is under cover: under
// two, both cut it.
float rounded_clip_alpha(float2 frag_pos)
{
  float alpha = 1.0;
  if (clip_active.x > 0.5)
  {
    alpha = rounded_clip_coverage(frag_pos, clip_rect, clip_radii_h, clip_radii_v);
  }
  if (clip_active.y > 0.5)
  {
    alpha *= rounded_clip_coverage(frag_pos, outer_clip_rect, outer_clip_radii, outer_clip_radii);
  }
  return alpha;
}

float rounded_fill_alpha(float2 local, float2 half_size, float4 radii_h, float4 radii_v)
{
  float2 radius = pick_radius(local, radii_h, radii_v);
  float dist = sd_rounded_box(local, half_size, radius);
  return saturate(0.5 - dist / subsample_aa_width(dist));
}

// Strokes. The same formulas as `layout/stroke.rs`, which describes the
// shape, and `quad.wgsl`; keep the three in step.

// How far the hole of a stroke reaches past an edge whose side has no
// stroke, so the ring has no coverage there.
static const float HOLE_MARGIN = 1.0;

float stroke_inset(float width)
{
  return width > 0.0 ? width : -HOLE_MARGIN;
}

// How much nearer the second of two edges that meet at a corner has to be for
// a point to be taken as that edge's.
static const float CORNER_BIAS = 0.01;

// Whether a point `first` away from one edge of a corner and `second` from
// the other is taken as the first edge's: the nearer one, and always the one
// that has a stroke where the other has none.
bool first_edge(float first, float second, float first_width, float second_width)
{
  if (second_width <= 0.0)
  {
    return true;
  }
  if (first_width <= 0.0)
  {
    return false;
  }
  return first <= second + CORNER_BIAS;
}

// Where along the rect's outline the part of it nearest `p` is, in pixels
// clockwise from the start of the top edge (`.x`), and the outline's whole
// length (`.y`). `radii` are circular, in the order TL, TR, BR, BL; `stroke`
// is the widths of the sides (top, right, bottom, left).
float2 outline_position(float2 p, float2 hs, float4 radii, float4 stroke)
{
  float limit = max(min(hs.x, hs.y), 0.0);
  float4 r = clamp(radii, float4(0.0, 0.0, 0.0, 0.0), float4(limit, limit, limit, limit));
  float quarter = 1.57079632679;
  float top = max(2.0 * hs.x - r.x - r.y, 0.0);
  float right = max(2.0 * hs.y - r.y - r.z, 0.0);
  float bottom = max(2.0 * hs.x - r.z - r.w, 0.0);
  float left = max(2.0 * hs.y - r.w - r.x, 0.0);
  // Where each part of the outline starts.
  float right_arc = top;
  float right_edge = right_arc + quarter * r.y;
  float bottom_arc = right_edge + right;
  float bottom_edge = bottom_arc + quarter * r.z;
  float left_arc = bottom_edge + bottom;
  float left_edge = left_arc + quarter * r.w;
  float top_arc = left_edge + left;
  float total = top_arc + quarter * r.x;

  float on_top = clamp(p.x + hs.x - r.x, 0.0, top);
  float on_right = right_edge + clamp(p.y + hs.y - r.y, 0.0, right);
  float on_bottom = bottom_edge + clamp(hs.x - r.z - p.x, 0.0, bottom);
  float on_left = left_edge + clamp(hs.y - r.w - p.y, 0.0, left);
  float position;
  if (p.y < 0.0)
  {
    if (p.x < 0.0)
    {
      float2 c = float2(-hs.x + r.x, -hs.y + r.x);
      if (p.x < c.x && p.y < c.y)
      {
        position = top_arc + atan2(c.y - p.y, c.x - p.x) * r.x;
      }
      else if (first_edge(p.x + hs.x, p.y + hs.y, stroke.w, stroke.x))
      {
        position = on_left;
      }
      else
      {
        position = on_top;
      }
    }
    else
    {
      float2 c = float2(hs.x - r.y, -hs.y + r.y);
      if (p.x > c.x && p.y < c.y)
      {
        position = right_arc + atan2(p.x - c.x, c.y - p.y) * r.y;
      }
      else if (first_edge(p.y + hs.y, hs.x - p.x, stroke.x, stroke.y))
      {
        position = on_top;
      }
      else
      {
        position = on_right;
      }
    }
  }
  else
  {
    if (p.x < 0.0)
    {
      float2 c = float2(-hs.x + r.w, hs.y - r.w);
      if (p.x < c.x && p.y > c.y)
      {
        position = left_arc + atan2(c.x - p.x, p.y - c.y) * r.w;
      }
      else if (first_edge(hs.y - p.y, p.x + hs.x, stroke.z, stroke.w))
      {
        position = on_bottom;
      }
      else
      {
        position = on_left;
      }
    }
    else
    {
      float2 c = float2(hs.x - r.z, hs.y - r.z);
      if (p.x > c.x && p.y > c.y)
      {
        position = bottom_arc + atan2(p.y - c.y, p.x - c.x) * r.z;
      }
      else if (first_edge(hs.x - p.x, hs.y - p.y, stroke.y, stroke.z))
      {
        position = on_right;
      }
      else
      {
        position = on_bottom;
      }
    }
  }
  return float2(position, total);
}

// How much of a dashed stroke shows at `position` along an outline of length
// `total`: 1 in a dash, 0 in a gap, a ramp of a pixel between. The period is
// fitted to the outline a whole number of times.
float dash_coverage(float position, float total, float dash, float gap)
{
  if (dash <= 0.0 || gap <= 0.0 || total <= 0.0)
  {
    return 1.0;
  }
  float period = dash + gap;
  float fitted = total / max(floor(total / period + 0.5), 1.0);
  float on = dash * fitted / period;
  float phase = position - floor(position / fitted) * fitted;
  float d = phase <= on ? -min(phase, on - phase) : min(phase - on, fitted - phase);
  return saturate(0.5 - d);
}

// How a stroke's opacity falls off along the arc of a rounded corner where
// its side meets a side with none: `on` is 1 where the arc leaves the side
// with the stroke and 0 where it reaches the other. The one place to tune how
// a border on some sides of a rounded element dissolves at its ends.
float corner_fade(float on)
{
  float t = saturate(on);
  return t * t * (3.0 - 2.0 * t);
}

// How the colors of two sides that meet at a corner mix across it: `turn` is
// 0 on the top or bottom side and 1 on the left or right one.
float corner_color_mix(float turn)
{
  float t = saturate(turn);
  return t * t * (3.0 - 2.0 * t);
}

// The sides that meet at the corner of `local`'s quadrant, as widths (`.x`
// across: the left or right side, `.y` along: the top or bottom one).
float2 quadrant_widths(float2 local, float4 widths)
{
  return float2(local.x < 0.0 ? widths.w : widths.y, local.y < 0.0 ? widths.x : widths.z);
}

// The widths the stroke has in a corner whose sides are `sides` wide (across,
// along): theirs, but at a rounded corner where one side has none the other's
// width goes on round the corner.
float2 corner_widths(float2 sides, float2 radius)
{
  if (radius.x <= 0.0 || radius.y <= 0.0)
  {
    return sides;
  }
  float width = max(sides.x, sides.y);
  return float2(sides.x <= 0.0 ? width : sides.x, sides.y <= 0.0 ? width : sides.y);
}

// The widths the ring has at `local`: the corner's widths in the arc of a
// rounded corner, the sides' own anywhere else, so that past the arc of a
// corner where the stroke ends the side with none has no ring at all and the
// distance to the ring does not jump where the quadrants meet.
float2 ring_widths(float2 local, float2 half_size, float2 radius, float2 sides, float radii_at)
{
  float2 corner = corner_widths(sides, radius);
  if (all(corner == sides))
  {
    return sides;
  }
  float2 edge = half_size + radii_at * (corner - sides);
  float2 centre = edge - (radius + radii_at * corner);
  if (all(abs(local) > centre))
  {
    return corner;
  }
  return sides;
}

// A box given by how far each of its edges (top, right, bottom, left) is
// inside the rect of half extent `half_size`: its centre (`.xy`) and half
// extent (`.zw`).
float4 inset_box(float2 half_size, float4 insets)
{
  float2 centre = float2(0.5 * (insets.w - insets.y), 0.5 * (insets.x - insets.z));
  float2 half_extent = max(float2(
    half_size.x - 0.5 * (insets.w + insets.y),
    half_size.y - 0.5 * (insets.x + insets.z)
  ), float2(0.0, 0.0));
  return float4(centre, half_extent);
}

// The ring of a stroke: inside the outer edge and outside the hole. `stroke`
// is the widths (top, right, bottom, left); `radii_at` says which edge of the
// stroke the radii are the corners of (0 outer, 1 the hole's). Along the arc
// of a rounded corner whose other side has no stroke the ring goes on at
// the same width, and reaches past that side as far as the stroke reaches
// past its own.
float rounded_stroke_alpha(float2 local, float2 half_size, float4 radii_h, float4 radii_v, float4 stroke, float radii_at)
{
  float4 widths = max(stroke, float4(0.0, 0.0, 0.0, 0.0));
  float2 radius = pick_radius(local, radii_h, radii_v);
  float2 sides = quadrant_widths(local, widths);
  float2 corner = ring_widths(local, half_size, radius, sides, radii_at);

  // How far the outer edge is past the rect on each side, and the hole's edge
  // inside it.
  float2 reach = radii_at * (corner - sides);
  float4 outward = float4(0.0, 0.0, 0.0, 0.0);
  float4 insets = float4(
    stroke_inset(widths.x),
    stroke_inset(widths.y),
    stroke_inset(widths.z),
    stroke_inset(widths.w)
  );
  if (local.x < 0.0)
  {
    outward.w = reach.x;
    insets.w = stroke_inset(corner.x);
  }
  else
  {
    outward.y = reach.x;
    insets.y = stroke_inset(corner.x);
  }
  if (local.y < 0.0)
  {
    outward.x = reach.y;
    insets.x = stroke_inset(corner.y);
  }
  else
  {
    outward.z = reach.y;
    insets.z = stroke_inset(corner.y);
  }

  float2 outer_radius = float2(0.0, 0.0);
  float2 hole_radius = float2(0.0, 0.0);
  if (radius.x > 0.0)
  {
    outer_radius.x = radius.x + radii_at * corner.x;
    hole_radius.x = max(outer_radius.x - stroke_inset(corner.x), 0.0);
  }
  if (radius.y > 0.0)
  {
    outer_radius.y = radius.y + radii_at * corner.y;
    hole_radius.y = max(outer_radius.y - stroke_inset(corner.y), 0.0);
  }
  float4 outer = inset_box(half_size, -outward);
  float outer_dist = sd_rounded_box(local - outer.xy, outer.zw, outer_radius);
  float4 hole = inset_box(half_size, insets - outward);
  float hole_dist = sd_rounded_box(local - hole.xy, hole.zw, hole_radius);
  float dist = max(outer_dist, -hole_dist);
  return saturate(0.5 - dist / subsample_aa_width(dist));
}

// Where `local` is round the corner of its quadrant: 0 on the top or bottom
// side, 1 on the left or right one, the angle in between along a rounded
// corner's arc. In a square corner, and inside the arc's centre of a stroke
// wider than its corner, the two sides meet on the line from the outer corner
// to the hole's, a pixel wide.
float corner_turn(float2 local, float2 half_size, float4 radii_h, float4 radii_v, float4 stroke, float radii_at)
{
  float4 widths = max(stroke, float4(0.0, 0.0, 0.0, 0.0));
  float2 radius = pick_radius(local, radii_h, radii_v);
  float2 sides = quadrant_widths(local, widths);
  float2 corner = corner_widths(sides, radius);
  // Mirrored into the bottom-right quadrant.
  float2 p = abs(local);
  float2 edge = half_size + radii_at * (corner - sides);
  float2 outer_radius = float2(0.0, 0.0);
  if (radius.x > 0.0 && radius.y > 0.0)
  {
    outer_radius = radius + radii_at * corner;
  }
  float2 d = p - (edge - outer_radius);
  if (d.x > 0.0 && d.y > 0.0 && outer_radius.x > 0.0)
  {
    return atan2(d.x / outer_radius.x, d.y / outer_radius.y) * 0.63661977;
  }
  if (d.x <= 0.0 && d.y > 0.0)
  {
    return 0.0;
  }
  if (d.x > 0.0 && d.y <= 0.0)
  {
    return 1.0;
  }
  float2 to_edge = edge - p;
  float span = max(length(corner), 1e-4);
  return saturate(0.5 + (to_edge.y * corner.x - to_edge.x * corner.y) / span);
}

// How much of the stroke shows at `local` along a rounded corner where one of
// the sides that meet has none: it fades out from the side with a stroke to
// the other. In a quadrant whose sides both have none, only the band of a far
// side wide enough to reach into it (more than half the rect) shows: not the
// end of an arc that goes on past the middle of a side, as round a pill whose
// stroke is outside it. 1 anywhere else.
float corner_fade_at(float2 local, float2 half_size, float4 radii_h, float4 radii_v, float4 stroke, float radii_at)
{
  float4 widths = max(stroke, float4(0.0, 0.0, 0.0, 0.0));
  float2 radius = pick_radius(local, radii_h, radii_v);
  float2 sides = quadrant_widths(local, widths);
  if (sides.x <= 0.0 && sides.y <= 0.0)
  {
    float2 far_widths = float2(local.x < 0.0 ? widths.y : widths.w, local.y < 0.0 ? widths.z : widths.x);
    // How far the point is from the far edges, less a pixel for their ramps.
    float2 from_far = half_size + abs(local) - float2(1.0, 1.0);
    return any(from_far < far_widths) ? 1.0 : 0.0;
  }
  if (radius.x <= 0.0 || radius.y <= 0.0 || (sides.x > 0.0) == (sides.y > 0.0))
  {
    return 1.0;
  }
  float turn = corner_turn(local, half_size, radii_h, radii_v, stroke, radii_at);
  return sides.y > 0.0 ? corner_fade(1.0 - turn) : corner_fade(turn);
}

// The stroke's color at `local` when its sides have colors of their own, four
// at `offset` in `gradients` (top, right, bottom, left): a side's own along
// it, mixed across a corner where two sides with a stroke meet.
float4 stroke_side_color(int offset, float2 local, float2 half_size, float4 radii_h, float4 radii_v, float4 stroke, float radii_at)
{
  float2 sides = quadrant_widths(local, max(stroke, float4(0.0, 0.0, 0.0, 0.0)));
  float4 across = gradients[offset + (local.x < 0.0 ? 3 : 1)];
  float4 along = gradients[offset + (local.y < 0.0 ? 0 : 2)];
  if (sides.y <= 0.0)
  {
    return across;
  }
  if (sides.x <= 0.0)
  {
    return along;
  }
  float turn = corner_turn(local, half_size, radii_h, radii_v, stroke, radii_at);
  return lerp(along, across, corner_color_mix(turn));
}

float supersampled_fill_alpha(float2 local, float2 half_size, float4 radii_h, float4 radii_v)
{
  return (
    rounded_fill_alpha(local + float2(-0.25, -0.25), half_size, radii_h, radii_v) +
    rounded_fill_alpha(local + float2(0.25, -0.25), half_size, radii_h, radii_v) +
    rounded_fill_alpha(local + float2(-0.25, 0.25), half_size, radii_h, radii_v) +
    rounded_fill_alpha(local + float2(0.25, 0.25), half_size, radii_h, radii_v)
  ) * 0.25;
}

float supersampled_stroke_alpha(float2 local, float2 half_size, float4 radii_h, float4 radii_v, float4 stroke, float radii_at)
{
  return (
    rounded_stroke_alpha(local + float2(-0.25, -0.25), half_size, radii_h, radii_v, stroke, radii_at) +
    rounded_stroke_alpha(local + float2(0.25, -0.25), half_size, radii_h, radii_v, stroke, radii_at) +
    rounded_stroke_alpha(local + float2(-0.25, 0.25), half_size, radii_h, radii_v, stroke, radii_at) +
    rounded_stroke_alpha(local + float2(0.25, 0.25), half_size, radii_h, radii_v, stroke, radii_at)
  ) * 0.25;
}

float2 shadow_erf(float2 x)
{
  float2 s = sign(x);
  float2 a = abs(x);
  float2 r = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
  r = r * r;
  return s - s / (r * r);
}

float shadow_gaussian(float x, float sigma)
{
  return exp(-(x * x) / (2.0 * sigma * sigma)) / (2.5066283 * sigma);
}

// A row of the rounded rect at height `y`, convolved along x with the
// Gaussian: the difference of two `erf`s at the row's curved ends.
float shadow_blur_along_x(float x, float y, float sigma, float corner, float2 half_size)
{
  float delta = min(half_size.y - corner - abs(y), 0.0);
  float curved = half_size.x - corner + sqrt(max(corner * corner - delta * delta, 0.0));
  float2 integral = 0.5 + 0.5 * shadow_erf((float2(x, x) + float2(-curved, curved)) * (0.70710678 / sigma));
  return integral.y - integral.x;
}

// Coverage of a rounded rect (half extent `half_size`, circular corner
// `radii`) blurred by `sigma`, at centre-relative `p`; the y convolution is a
// midpoint sum over +/- 3 sigma.
float blurred_rounded_box(float2 p, float2 half_size, float4 radii, float sigma)
{
  if (half_size.x <= 0.0 || half_size.y <= 0.0)
  {
    return 0.0;
  }
  float corner = max(min(pick_radius(p, radii, radii).x, min(half_size.x, half_size.y)), 0.0);
  float extent = SHADOW_EXTENT_SIGMAS * sigma;
  float start = clamp(-extent, p.y - half_size.y, p.y + half_size.y);
  float end = clamp(extent, p.y - half_size.y, p.y + half_size.y);
  float step = (end - start) / (float)SHADOW_Y_SAMPLES;
  float y = start + step * 0.5;
  float coverage = 0.0;
  [unroll]
  for (int i = 0; i < SHADOW_Y_SAMPLES; i = i + 1)
  {
    coverage += shadow_blur_along_x(p.x, p.y - y, sigma, corner, half_size) * shadow_gaussian(y, sigma) * step;
    y += step;
  }
  return saturate(coverage);
}

float shadow_shape_alpha(float2 p, float2 half_size, float4 radii, float sigma)
{
  if (sigma < SHADOW_MIN_SIGMA)
  {
    if (half_size.x <= 0.0 || half_size.y <= 0.0)
    {
      return 0.0;
    }
    return supersampled_fill_alpha(p, half_size, radii, radii);
  }
  return blurred_rounded_box(p, half_size, radii, sigma);
}

// An outer shadow covers its shape outside the element box; an inset one
// covers the box outside its shape.
float box_shadow_alpha(float2 local, float2 half_size, float4 box_radii, float4 shape_radii, float4 params, float sigma)
{
  float2 offset = params.yz;
  float spread = params.w;
  float box_alpha = supersampled_fill_alpha(local, half_size, box_radii, box_radii);
  if (params.x < 1.5)
  {
    float2 shape_half = max(half_size + float2(spread, spread), float2(0.0, 0.0));
    return shadow_shape_alpha(local - offset, shape_half, shape_radii, sigma) * (1.0 - box_alpha);
  }
  float2 shape_half = max(half_size - float2(spread, spread), float2(0.0, 0.0));
  return box_alpha * (1.0 - shadow_shape_alpha(local - offset, shape_half, shape_radii, sigma));
}

// The render target is not sRGB, so the fixed-function blend mixes
// sRGB-encoded values, as CSS and design tools do. `ps_main` works in linear
// light like the rest of the pipeline and encodes each result it returns.
float4 encode_srgb(float4 color)
{
  float3 c = saturate(color.rgb);
  float3 low = c * 12.92;
  float3 high = 1.055 * pow(c, 1.0 / 2.4) - 0.055;
  return float4(c <= 0.0031308 ? low : high, color.a);
}

float4 ps_main(VsOut input) : SV_TARGET
{
  float clip_alpha_value = rounded_clip_alpha(input.position.xy);
  if (clip_alpha_value <= 0.0)
  {
    discard;
  }

  float max_stroke = max(max(input.stroke.x, input.stroke.y), max(input.stroke.z, input.stroke.w));
  float4 base_color = input.color;
  if (max_stroke <= 0.0)
  {
    if (input.gradient_offset >= 0.0)
    {
      int off = (int)(input.gradient_offset + 0.5);
      if ((int)gradients[off].x >= 1)
      {
        base_color = sample_gradient(off, input.local, input.half_size);
      }
    }
    if (input.pattern.x > 0.5)
    {
      float shadow_alpha = box_shadow_alpha(input.local, input.half_size, input.radii_h, input.radii_v, input.pattern, input.shadow_sigma);
      if (shadow_alpha <= 0.0)
      {
        discard;
      }
      return encode_srgb(float4(base_color.rgb, base_color.a * shadow_alpha * clip_alpha_value));
    }
    float fill_alpha = supersampled_fill_alpha(input.local, input.half_size, input.radii_h, input.radii_v);
    return encode_srgb(float4(base_color.rgb, base_color.a * fill_alpha * clip_alpha_value));
  }

  // A stroke's pattern holds its dash (y), its gap (z) and the edge its radii are at (w).
  float radii_at = input.pattern.w;
  float alpha = supersampled_stroke_alpha(input.local, input.half_size, input.radii_h, input.radii_v, input.stroke, radii_at);
  if (alpha > 0.0)
  {
    alpha *= corner_fade_at(input.local, input.half_size, input.radii_h, input.radii_v, input.stroke, radii_at);
  }
  if (alpha > 0.0 && input.pattern.y > 0.0 && input.pattern.z > 0.0)
  {
    float2 place = outline_position(input.local, input.half_size, input.radii_h, input.stroke);
    alpha *= dash_coverage(place.x, place.y, input.pattern.y, input.pattern.z);
  }
  // A stroke whose sides differ in color has the four (top, right, bottom,
  // left) in `gradients`, where a fill keeps its gradient.
  if (alpha > 0.0 && input.gradient_offset >= 0.0)
  {
    int off = (int)(input.gradient_offset + 0.5);
    base_color = stroke_side_color(off, input.local, input.half_size, input.radii_h, input.radii_v, input.stroke, radii_at);
  }
  return encode_srgb(float4(base_color.rgb, base_color.a * alpha * clip_alpha_value));
}
