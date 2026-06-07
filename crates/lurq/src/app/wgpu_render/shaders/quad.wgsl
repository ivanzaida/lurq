// Rounded-corner quad pipeline (elliptical, optional ring).
//
// Vertex buffer 0 (per-vertex):  unit quad corner in [0,1]^2.
// Vertex buffer 1 (per-instance): pos / size in pixels, linear RGBA color,
//                                 horizontal corner radii (TL, TR, BR, BL),
//                                 vertical corner radii (same order),
//                                 per-side ring thickness (top, right,
//                                 bottom, left).
// Bind group 0 / binding 0: viewport size in pixels.
//
// Two modes selected by whether any stroke component is > 0:
//   - Filled:    paint the entire (rounded) box with `color`.
//   - Stroked:   paint only the ring between the outer rounded box and
//                an inner rounded box inset on each side by the matching
//                stroke width.
// In both modes a ~1-pixel anti-alias band keeps edges smooth.
//
// The corner zone uses a gradient-corrected ellipse SDF so corners can
// be elliptical (h != v); when h == v it reduces to the usual circular
// case.

// `Globals` is a per-clip-range uniform block — `prepare` writes one
// entry per `DisplayList::clips` slot and `record` rebinds it via a
// dynamic offset. Vertex stage reads `viewport.xy` for NDC mapping;
// fragment stage uses `clip_*` to discard fragments outside the
// active rounded clip.
struct Globals {
    viewport:     vec4<f32>,
    clip_rect:    vec4<f32>,   // x, y, w, h
    clip_radii_h: vec4<f32>,
    clip_radii_v: vec4<f32>,
    clip_active:  vec4<f32>,   // x = 1.0 to enable SDF discard
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read> gradients: array<vec4<f32>>;

struct VsIn {
    @location(0) corner:     vec2<f32>,
    @location(1) pos:        vec2<f32>,
    @location(2) size:       vec2<f32>,
    @location(3) color:      vec4<f32>,
    @location(4) radii_h:    vec4<f32>,  // TL, TR, BR, BL
    @location(5) radii_v:    vec4<f32>,  // TL, TR, BR, BL
    @location(6) stroke:     vec4<f32>,  // top, right, bottom, left
    @location(7) pattern:    vec4<f32>,  // kind, dash, gap, _
    @location(8) transform:     vec4<f32>,  // 2x2 matrix: a, b, c, d
    @location(9) xf_origin:    vec2<f32>,  // transform origin relative to rect top-left
    @location(10) shadow_sigma: f32,
    @location(11) gradient_offset: f32,
};

struct VsOut {
    @builtin(position) clip:      vec4<f32>,
    @location(0)       color:     vec4<f32>,
    /// Pixel offset from the box's centre.
    @location(1)       local:     vec2<f32>,
    @location(2)       half_size: vec2<f32>,
    @location(3)       radii_h:   vec4<f32>,
    @location(4)       radii_v:   vec4<f32>,
    @location(5)       stroke:       vec4<f32>,
    @location(6)       pattern:      vec4<f32>,
    @location(7)       shadow_sigma: f32,
    @location(8)       gradient_offset: f32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    // Local position within the quad (before transform). The mesh is
    // expanded slightly so transformed edges have fragments on both
    // sides of the SDF boundary; otherwise the rasterizer clips away
    // the outer half of the AA band.
    let aa_outset = 2.0;
    let local_px = in.corner * (in.size + vec2<f32>(aa_outset * 2.0, aa_outset * 2.0))
                 - vec2<f32>(aa_outset, aa_outset);
    // Apply 2x2 transform around xf_origin.
    let centered = local_px - in.xf_origin;
    let rotated = vec2<f32>(
        in.transform.x * centered.x + in.transform.z * centered.y,
        in.transform.y * centered.x + in.transform.w * centered.y,
    );
    let px = in.pos + rotated + in.xf_origin;

    let viewport = globals.viewport.xy;
    let ndc = vec2<f32>(
        (px.x / viewport.x) * 2.0 - 1.0,
        1.0 - (px.y / viewport.y) * 2.0,
    );

    var out: VsOut;
    out.clip      = vec4<f32>(ndc, 0.0, 1.0);
    out.color     = in.color;
    out.half_size = in.size * 0.5;
    // SDF local coords stay in un-rotated quad space.
    out.local     = local_px - in.size * 0.5;
    out.radii_h   = in.radii_h;
    out.radii_v   = in.radii_v;
    out.stroke       = in.stroke;
    out.pattern      = in.pattern;
    out.shadow_sigma     = in.shadow_sigma;
    out.gradient_offset  = in.gradient_offset;
    return out;
}

/// Arc-length along the outline of a uniform-radius rounded box, from
/// the start of the side's 45° wedge to the projection of `p` onto the
/// outline. Only used for one-sided rings with a dash/dot pattern.
///
/// `side_idx`: 0 = top, 1 = right, 2 = bottom, 3 = left.
/// `r`:        circular corner radius (assumes h == v on every corner).
///
/// Each side spans: half of the entering corner arc + the straight
/// edge + half of the exiting corner arc. The result is in pixels and
/// monotonically increases as we walk the outline within the wedge.
fn perimeter_param(p: vec2<f32>, hs: vec2<f32>, r: f32, side_idx: i32) -> f32 {
    let pi = 3.14159265359;
    let quarter = pi * r * 0.5;
    let half_quarter = quarter * 0.5;

    // Map the fragment into the canonical "top side" frame so we only
    // implement the geometry once. After this swap, `q` is in the same
    // coordinate space as if `side_idx` were 0 (top), and the entering
    // corner is at the top-left, exiting corner at the top-right.
    var q: vec2<f32>;
    var sz: vec2<f32>;
    switch side_idx {
        case 0: { q = p;                              sz = hs;                          }
        case 1: { q = vec2<f32>( p.y,        -p.x);   sz = vec2<f32>(hs.y, hs.x);       }
        case 2: { q = vec2<f32>(-p.x,        -p.y);   sz = hs;                          }
        case 3: { q = vec2<f32>(-p.y,         p.x);   sz = vec2<f32>(hs.y, hs.x);       }
        default: { q = p; sz = hs; }
    }

    let straight_len = max(2.0 * sz.x - 2.0 * r, 0.0);
    let total = half_quarter + straight_len + half_quarter;

    // Entering corner zone: q.x < -sz.x + r AND q.y near top.
    let left_corner = vec2<f32>(-sz.x + r, -sz.y + r);
    let right_corner = vec2<f32>(sz.x - r, -sz.y + r);

    if (q.x < left_corner.x) {
        // Top half of TL arc. Theta sweeps from -3π/4 (wedge boundary)
        // to -π/2 (top tangent). Param goes 0 → half_quarter.
        let v = q - left_corner;
        let theta = atan2(v.y, v.x);
        let t = clamp(theta - (-0.75 * pi), 0.0, 0.25 * pi);
        return t * r;
    } else if (q.x > right_corner.x) {
        // Top half of TR arc. Theta sweeps from -π/2 to -π/4.
        let v = q - right_corner;
        let theta = atan2(v.y, v.x);
        let t = clamp(theta - (-0.5 * pi), 0.0, 0.25 * pi);
        return half_quarter + straight_len + t * r;
    }
    // Straight portion of the top edge.
    let s = clamp(q.x - left_corner.x, 0.0, straight_len);
    return half_quarter + s;
}

/// Pick the (h, v) radius pair for whichever quadrant `p` lies in.
/// Order: TL, TR, BR, BL (matches CSS `border-radius` longhand order).
fn pick_radius(p: vec2<f32>, rh: vec4<f32>, rv: vec4<f32>) -> vec2<f32> {
    if (p.y < 0.0) {
        if (p.x < 0.0) { return vec2<f32>(rh.x, rv.x); } // TL
        else           { return vec2<f32>(rh.y, rv.y); } // TR
    } else {
        if (p.x < 0.0) { return vec2<f32>(rh.w, rv.w); } // BL
        else           { return vec2<f32>(rh.z, rv.z); } // BR
    }
}

/// SDF of a box with elliptical corners (radius `r = (rx, ry)` per
/// quadrant). `p` is centre-relative pixel coords; `half_size` is the
/// box half-extent.
///
/// - Corner zone (both q components > 0) → gradient-corrected ellipse
///   SDF. Reduces to the exact circular formula `length(q) - r` when
///   rx == ry; smooth approximation otherwise.
/// - Edge band / interior → rectangle distance with the corner radii
///   subtracted on each axis: `max(q.x - rx, q.y - ry)`.
fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, r: vec2<f32>) -> f32 {
    let safe_r = max(r, vec2<f32>(0.0, 0.0));
    let q = abs(p) - half_size + safe_r;

    if (q.x > 0.0 && q.y > 0.0) {
        // Corner zone.
        if (safe_r.x <= 0.0 || safe_r.y <= 0.0) {
            return max(q.x, q.y);
        }
        let pn = q / safe_r;
        let l = length(pn);
        if (l <= 1e-6) {
            return -min(safe_r.x, safe_r.y);
        }
        let g = max(length(pn / safe_r), 1e-6);
        // Euclidean distance estimate = (length(pn) - 1) / |grad|, where
        // |grad| = length(pn / r) / length(pn). The factor of `l` in the
        // numerator below absorbs the `length(pn)` in the denominator
        // of |grad|. Reduces to `length(q) - r` for circular corners.
        return (l - 1.0) * l / g;
    }

    // Edge band or interior. The `- safe_r.{x,y}` subtraction undoes the
    // `+ safe_r` we added when computing `q`, which keeps the corner
    // zone consistent with the rounded-box shape.
    return max(q.x - safe_r.x, q.y - safe_r.y);
}

/// Evaluate a CSS-like gradient for a fragment at centre-relative pixel
/// `local`, within a box of half-extent `half`. `off` is the vec4 index of
/// the gradient header in the `gradients` storage buffer. Layout:
///   [count, kind, flags, from_angle]
///   [dir.x, dir.y, center.x, center.y]
///   then per stop: [r, g, b, a], [position, _, _, _]
/// kind: 0 = linear, 1 = radial (flags bit0 = ellipse), 2 = conic.
fn sample_gradient(off: i32, local: vec2<f32>, half: vec2<f32>) -> vec4<f32> {
    let h0 = gradients[off];
    let h1 = gradients[off + 1];
    let count = i32(h0.x);
    let kind = i32(h0.y);
    let pi = 3.14159265359;

    var t: f32;
    if (kind == 0) {
        // Linear: project onto the unit direction, normalized by half the
        // gradient-line length so 0/1 reach the corners (CSS behaviour).
        let dir = h1.xy;
        let hl = abs(half.x * dir.x) + abs(half.y * dir.y);
        t = (dot(local, dir) + hl) / (2.0 * max(hl, 1e-5));
    } else if (kind == 1) {
        // Radial, farthest-corner.
        let center = (h1.zw * 2.0 - vec2<f32>(1.0, 1.0)) * half;
        if (h0.z > 0.5) {
            // Ellipse fitted to the box.
            let cn = h1.zw * 2.0 - vec2<f32>(1.0, 1.0);
            let p = (local - center) / max(half, vec2<f32>(1e-3, 1e-3));
            let radius = max(
                max(length(vec2<f32>(-1.0, -1.0) - cn), length(vec2<f32>(1.0, -1.0) - cn)),
                max(length(vec2<f32>(-1.0, 1.0) - cn), length(vec2<f32>(1.0, 1.0) - cn)),
            );
            t = length(p) / max(radius, 1e-5);
        } else {
            let radius = max(
                max(length(vec2<f32>(-half.x, -half.y) - center), length(vec2<f32>(half.x, -half.y) - center)),
                max(length(vec2<f32>(-half.x, half.y) - center), length(vec2<f32>(half.x, half.y) - center)),
            );
            t = length(local - center) / max(radius, 1e-5);
        }
    } else {
        // Conic: angle clockwise from the top.
        let center = (h1.zw * 2.0 - vec2<f32>(1.0, 1.0)) * half;
        let d = local - center;
        let ang = (atan2(d.x, -d.y) - h0.w) / (2.0 * pi);
        t = ang - floor(ang);
    }

    if (kind != 2) {
        t = clamp(t, 0.0, 1.0);
    }

    let stop_base = off + 2;
    let last = count - 1;
    var color = gradients[stop_base + 2 * last];
    for (var i: i32 = 0; i < last; i = i + 1) {
        let pb = gradients[stop_base + 2 * (i + 1) + 1].x;
        if (t <= pb) {
            let pa = gradients[stop_base + 2 * i + 1].x;
            let ca = gradients[stop_base + 2 * i];
            let cb = gradients[stop_base + 2 * (i + 1)];
            let span = max(pb - pa, 1e-5);
            color = mix(ca, cb, clamp((t - pa) / span, 0.0, 1.0));
            break;
        }
    }
    return color;
}

/// Count how many sides have a positive stroke width.
fn nonzero_side_count(s: vec4<f32>) -> i32 {
    var n: i32 = 0;
    if (s.x > 0.0) { n = n + 1; }
    if (s.y > 0.0) { n = n + 1; }
    if (s.z > 0.0) { n = n + 1; }
    if (s.w > 0.0) { n = n + 1; }
    return n;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var clip_alpha = 1.0;
    // Rounded-clip discard. The active clip's rect + radii are
    // supplied by the per-range `Globals` uniform; we recover the
    // fragment's screen-space pixel position from the @builtin
    // (which the rasteriser fills as the un-normalised pixel
    // coord) and reuse the same SDF helper as the box itself uses
    // for its own corners.
    if (globals.clip_active.x > 0.5) {
        let frag_pos = in.clip.xy;
        let cr = globals.clip_rect;
        let half = vec2<f32>(cr.z, cr.w) * 0.5;
        let centre = vec2<f32>(cr.x + half.x, cr.y + half.y);
        let local_clip = frag_pos - centre;
        let r = pick_radius(local_clip, globals.clip_radii_h, globals.clip_radii_v);
        let d = sd_rounded_box(local_clip, half, r);
        let aa = max(fwidth(d), 0.001);
        clip_alpha = clamp(0.5 - d / aa, 0.0, 1.0);
        if (clip_alpha <= 0.0) {
            discard;
        }
    }

    var base_color = in.color;
    if (in.gradient_offset >= 0.0) {
        let off = i32(in.gradient_offset + 0.5);
        if (i32(gradients[off].x) >= 1) {
            base_color = sample_gradient(off, in.local, in.half_size);
        }
    }

    let outer_r = pick_radius(in.local, in.radii_h, in.radii_v);
    let outer_dist = sd_rounded_box(in.local, in.half_size, outer_r);

    let max_stroke = max(max(in.stroke.x, in.stroke.y), max(in.stroke.z, in.stroke.w));

    if (max_stroke <= 0.0) {
        // Shadow mode: soft Gaussian-like falloff.
        let sigma = in.shadow_sigma;
        if (sigma > 0.0) {
            let t = clamp(-outer_dist / sigma, 0.0, 1.0);
            let alpha = t * t * (3.0 - 2.0 * t) * base_color.a * clip_alpha;
            if (alpha <= 0.001) { discard; }
            return vec4<f32>(base_color.rgb, alpha);
        }
        if (sigma < 0.0) {
            // Inset shadow: fade inward from an inner reference shape.
            // pattern.xy = offset, pattern.z = spread.
            let sigma_abs = -sigma;
            let offset = in.pattern.xy;
            let spread = in.pattern.z;
            let inner_half = in.half_size - vec2<f32>(spread, spread);
            let inner_local = in.local - offset;
            let inner_r_raw = pick_radius(inner_local, in.radii_h, in.radii_v);
            let inner_r = max(inner_r_raw - vec2<f32>(spread, spread), vec2<f32>(0.0, 0.0));
            let inner_dist = sd_rounded_box(inner_local, inner_half, inner_r);
            let aa = max(fwidth(outer_dist), 0.001);
            let t = clamp(inner_dist / sigma_abs, 0.0, 1.0);
            let outer_mask = clamp(0.5 - outer_dist / aa, 0.0, 1.0);
            let alpha = t * t * (3.0 - 2.0 * t) * outer_mask * base_color.a * clip_alpha;
            if (alpha <= 0.001) { discard; }
            return vec4<f32>(base_color.rgb, alpha);
        }
        // Filled mode.
        let aa = max(fwidth(outer_dist), 0.001);
        let alpha = clamp(0.5 - outer_dist / aa, 0.0, 1.0);
        if (alpha <= 0.0) { discard; }
        return vec4<f32>(base_color.rgb, base_color.a * alpha * clip_alpha);
    }

    let nz = nonzero_side_count(in.stroke);
    let inner_half = vec2<f32>(
        in.half_size.x - 0.5 * (in.stroke.y + in.stroke.w),
        in.half_size.y - 0.5 * (in.stroke.x + in.stroke.z),
    );
    let inner_centre = vec2<f32>(
        0.5 * (in.stroke.w - in.stroke.y),
        0.5 * (in.stroke.x - in.stroke.z),
    );

    let inner_r = vec2<f32>(
        max(0.0, outer_r.x - max_stroke),
        max(0.0, outer_r.y - max_stroke),
    );
    let inner_dist = sd_rounded_box(in.local - inner_centre, inner_half, inner_r);
    let dist = max(outer_dist, -inner_dist);
    let aa = max(fwidth(dist), 0.001);

    var side_idx: i32 = -1;
    if (nz == 1) {
        if (in.stroke.x > 0.0) { side_idx = 0; } // top
        if (in.stroke.y > 0.0) { side_idx = 1; } // right
        if (in.stroke.z > 0.0) { side_idx = 2; } // bottom
        if (in.stroke.w > 0.0) { side_idx = 3; } // left
    }

    var alpha = clamp(0.5 - dist / aa, 0.0, 1.0);

    // Dash / dot modulation. Only meaningful on one-sided rings with a
    // uniform circular corner radius (h == v on every corner). Other
    // configurations leave the pattern unhonoured.
    let pattern_kind = in.pattern.x;
    if (side_idx >= 0 && pattern_kind > 0.5) {
        // Treat as circular if h ~= v on the relevant corners. Otherwise
        // keep solid.
        let r_max = max(max(in.radii_h.x, in.radii_h.y), max(in.radii_h.z, in.radii_h.w));
        let r_min = min(min(in.radii_h.x, in.radii_h.y), min(in.radii_h.z, in.radii_h.w));
        let v_max = max(max(in.radii_v.x, in.radii_v.y), max(in.radii_v.z, in.radii_v.w));
        let circular = abs(r_max - v_max) < 0.001 && abs(r_max - r_min) < 0.001;
        if (circular) {
            let r = r_max;
            let arc = perimeter_param(in.local, in.half_size, r, side_idx);
            let dash = max(in.pattern.y, 0.0001);
            let gap  = max(in.pattern.z, 0.0001);
            let period = dash + gap;
            let phase = arc - floor(arc / period) * period;
            let edge_aa = aa;
            // Smooth on at phase=0, smooth off at phase=dash.
            let dash_alpha = clamp(0.5 + (dash - phase) / edge_aa, 0.0, 1.0)
                           * clamp(0.5 + phase / edge_aa, 0.0, 1.0);
            alpha = alpha * dash_alpha;
        }
    }

    if (alpha <= 0.0) {
        discard;
    }
    return vec4<f32>(base_color.rgb, base_color.a * alpha * clip_alpha);
}
