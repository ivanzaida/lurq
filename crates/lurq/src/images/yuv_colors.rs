//! How the samples of a frame of gray and color (NV12, I420) are made red, green and blue: the matrix it was
//! made with and the range its samples take. A decoder knows both and says them; the shaders are told them
//! (`app/wgpu_render/shaders/yuv.wgsl`, `app/dx12_render/shaders/image_nv12.hlsl`).

/// The matrix a frame's gray and color were made of red, green and blue with: the standard's K_R and K_B.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum YuvMatrix {
  /// ITU-R BT.709: a video of high definition, and what is assumed where a decoder says nothing.
  #[default]
  Bt709,
  /// ITU-R BT.601: a video of standard definition, and most of what a camera of a phone or a JPEG gives.
  Bt601,
  /// ITU-R BT.2020, of a luminance that is not constant: a video of ultra high definition, HDR's.
  Bt2020,
}

/// How a frame's samples are colors: its matrix, and whether they take all of their range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct YuvColors {
  pub matrix: YuvMatrix,
  /// The samples take all of their range (0 to 255), and not gray from 16 to 235 and color from 16 to 240 (the
  /// video range, which a decoder gives unless it is told otherwise).
  pub full_range: bool,
}

impl YuvColors {
  /// BT.709 of the video range: what a frame is drawn as where nothing says otherwise.
  pub const BT709: Self = Self {
    matrix: YuvMatrix::Bt709,
    full_range: false,
  };

  /// A frame of `matrix` in the video range.
  pub fn new(matrix: YuvMatrix) -> Self {
    Self {
      matrix,
      full_range: false,
    }
  }

  /// The same, with samples that take all of their range.
  pub fn full_range(mut self) -> Self {
    self.full_range = true;
    self
  }

  /// What the shaders are told, in an image instance's `opacity.zw`: the matrix (0 BT.709, 1 BT.601, 2 BT.2020),
  /// and 1 for the full range.
  #[cfg(any(feature = "wgpu", feature = "dx12"))]
  pub(crate) fn shader(self) -> [f32; 2] {
    let matrix = match self.matrix {
      YuvMatrix::Bt709 => 0.0,
      YuvMatrix::Bt601 => 1.0,
      YuvMatrix::Bt2020 => 2.0,
    };
    [matrix, if self.full_range { 1.0 } else { 0.0 }]
  }
}
