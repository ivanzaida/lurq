
// The color of a sample of a video's frame: Y'CbCr made R'G'B' (gamma-encoded, 0 to 1), as the frame says it is.
// `matrix` is the one the frame was made with: 0 BT.709, 1 BT.601, 2 BT.2020 (of a constant luminance not).
// `full_range` is 1 where the samples take all of their range, 0 where gray is from 16 to 235 and color from 16 to 240
// of 255 (the video range, which a decoder gives unless told otherwise). Coefficients from the standards' K_R and K_B.
fn yuv_to_rgb(y: f32, cb_sample: f32, cr_sample: f32, matrix: f32, full_range: f32) -> vec3<f32> {
    var luma = y;
    var cb = cb_sample - 0.5;
    var cr = cr_sample - 0.5;
    if (full_range < 0.5) {
        luma = max(y - 16.0 / 255.0, 0.0) * (255.0 / 219.0);
        cb = cb * (255.0 / 224.0);
        cr = cr * (255.0 / 224.0);
    }
    // BT.709.
    var red_cr = 1.5748;
    var green_cb = 0.187324;
    var green_cr = 0.468124;
    var blue_cb = 1.8556;
    if (matrix > 1.5) {
        red_cr = 1.4746;
        green_cb = 0.164553;
        green_cr = 0.571353;
        blue_cb = 1.8814;
    } else if (matrix > 0.5) {
        red_cr = 1.402;
        green_cb = 0.344136;
        green_cr = 0.714136;
        blue_cb = 1.772;
    }
    return vec3<f32>(luma + red_cr * cr, luma - green_cb * cb - green_cr * cr, luma + blue_cb * cb);
}
