//! The QR code of a text (ISO/IEC 18004): its bytes in byte mode, in the smallest version that holds them at the
//! level of correction asked for, with the mask that scores best. The method is the one of Project Nayuki's
//! QR Code generator, which is the reference most encoders are checked against.

use super::QrLevel;

/// The codewords of correction in each block, by level and version (index 0 is no version).
const ECC_CODEWORDS_PER_BLOCK: [[u8; 41]; 4] = [
  [
    0, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30,
    30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
  ],
  [
    0, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28,
    28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
  ],
  [
    0, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30,
    30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
  ],
  [
    0, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30,
    30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
  ],
];

/// How many blocks the codewords are cut into, by level and version (index 0 is no version).
const ERROR_CORRECTION_BLOCKS: [[u8; 41]; 4] = [
  [
    0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19,
    19, 20, 21, 22, 24, 25,
  ],
  [
    0, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31,
    33, 35, 37, 38, 40, 43, 45, 47, 49,
  ],
  [
    0, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43,
    45, 48, 51, 53, 56, 59, 62, 65, 68,
  ],
  [
    0, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48,
    51, 54, 57, 60, 63, 66, 70, 74, 77, 81,
  ],
];

const PENALTY_N1: i32 = 3;
const PENALTY_N2: i32 = 3;
const PENALTY_N3: i32 = 40;
const PENALTY_N4: i32 = 10;

/// The modules of a QR code: which are dark, row by row, with no quiet zone around them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QrMatrix {
  size: usize,
  version: u8,
  level: QrLevel,
  modules: Vec<bool>,
}

impl QrMatrix {
  /// The code of `text`'s UTF-8 bytes at `level` or higher: the smallest version (1 to 40) that holds them at
  /// `level`, at the highest level that still fits in that version. `None` where no version holds them: more
  /// than 2953 bytes at `Low`, 1273 at `High`.
  pub fn encode(text: &str, level: QrLevel) -> Option<Self> {
    let data = text.as_bytes();
    let version = (1..=40u8).find(|&version| fits(data.len(), version, level))?;
    // A higher level costs nothing where it fits in the same version.
    let level = [QrLevel::Medium, QrLevel::Quartile, QrLevel::High]
      .into_iter()
      .filter(|higher| higher.index() > level.index() && fits(data.len(), version, *higher))
      .last()
      .unwrap_or(level);
    let codewords = add_correction_and_interleave(&data_codewords(data, version, level), version, level);
    let mut matrix = Grid::new(version);
    matrix.draw_function_patterns(level);
    matrix.draw_codewords(&codewords);
    let mask = (0..8u8)
      .min_by_key(|&mask| {
        matrix.apply_mask(mask);
        matrix.draw_format_bits(level, mask);
        let penalty = matrix.penalty();
        matrix.apply_mask(mask);
        penalty
      })
      .unwrap_or(0);
    matrix.apply_mask(mask);
    matrix.draw_format_bits(level, mask);
    Some(Self {
      size: matrix.size,
      version,
      level,
      modules: matrix.modules,
    })
  }

  /// How many modules a side has: 21 for version 1, four more for each version after it.
  pub fn size(&self) -> usize {
    self.size
  }

  /// The version, from 1 to 40.
  pub fn version(&self) -> u8 {
    self.version
  }

  /// The level the code was made at: the one asked for, or a higher one that fit in the same version.
  pub fn level(&self) -> QrLevel {
    self.level
  }

  /// Whether the module in column `x` of row `y` is dark. Outside the code it is light, as the quiet zone is.
  pub fn dark(&self, x: usize, y: usize) -> bool {
    x < self.size && y < self.size && self.modules[y * self.size + x]
  }
}

/// How many bits of the code's bit stream a text of `bytes` takes in byte mode at `version`.
fn bits_needed(bytes: usize, version: u8) -> usize {
  let count_bits = if version <= 9 { 8 } else { 16 };
  if bytes >= 1 << count_bits {
    return usize::MAX;
  }
  4 + count_bits + 8 * bytes
}

fn fits(bytes: usize, version: u8, level: QrLevel) -> bool {
  bits_needed(bytes, version) <= data_codeword_count(version, level) * 8
}

/// How many modules of `version` carry codewords: all but the function patterns and the format and version bits.
fn raw_data_modules(version: u8) -> usize {
  let version = usize::from(version);
  let mut modules = (16 * version + 128) * version + 64;
  if version >= 2 {
    let alignments = version / 7 + 2;
    modules -= (25 * alignments - 10) * alignments - 55;
    if version >= 7 {
      modules -= 36;
    }
  }
  modules
}

fn data_codeword_count(version: u8, level: QrLevel) -> usize {
  let (v, l) = (usize::from(version), level.index());
  raw_data_modules(version) / 8
    - usize::from(ECC_CODEWORDS_PER_BLOCK[l][v]) * usize::from(ERROR_CORRECTION_BLOCKS[l][v])
}

/// The data codewords: the mode, the count, the bytes, the terminator and the pad bytes.
fn data_codewords(data: &[u8], version: u8, level: QrLevel) -> Vec<u8> {
  let capacity = data_codeword_count(version, level) * 8;
  let mut bits = Bits::default();
  bits.push(0b0100, 4);
  bits.push(data.len() as u32, if version <= 9 { 8 } else { 16 });
  for byte in data {
    bits.push(u32::from(*byte), 8);
  }
  let terminator = (capacity - bits.len).min(4);
  bits.push(0, terminator);
  bits.push(0, (8 - bits.len % 8) % 8);
  let mut pad = [0xEC, 0x11].into_iter().cycle();
  while bits.len < capacity {
    bits.push(pad.next().unwrap_or(0xEC), 8);
  }
  bits.bytes
}

#[derive(Default)]
struct Bits {
  bytes: Vec<u8>,
  len: usize,
}

impl Bits {
  fn push(&mut self, value: u32, count: usize) {
    for index in (0..count).rev() {
      if self.len % 8 == 0 {
        self.bytes.push(0);
      }
      if (value >> index) & 1 != 0 {
        let at = self.len / 8;
        self.bytes[at] |= 0x80 >> (self.len % 8);
      }
      self.len += 1;
    }
  }
}

/// The codewords cut into blocks, each with its Reed-Solomon correction, read across the blocks.
fn add_correction_and_interleave(data: &[u8], version: u8, level: QrLevel) -> Vec<u8> {
  let (v, l) = (usize::from(version), level.index());
  let blocks = usize::from(ERROR_CORRECTION_BLOCKS[l][v]);
  let ecc_len = usize::from(ECC_CODEWORDS_PER_BLOCK[l][v]);
  let raw = raw_data_modules(version) / 8;
  let short_blocks = blocks - raw % blocks;
  let short_len = raw / blocks;
  let divisor = reed_solomon_divisor(ecc_len);
  let mut cut = Vec::with_capacity(blocks);
  let mut at = 0;
  for index in 0..blocks {
    let len = short_len - ecc_len + usize::from(index >= short_blocks);
    let mut block = data[at..at + len].to_vec();
    at += len;
    let ecc = reed_solomon_remainder(&block, &divisor);
    if index < short_blocks {
      block.push(0);
    }
    block.extend(ecc);
    cut.push(block);
  }
  let mut result = Vec::with_capacity(raw);
  for position in 0..cut[0].len() {
    for (index, block) in cut.iter().enumerate() {
      // The short blocks' hole, where the long ones have one more data codeword.
      if position != short_len - ecc_len || index >= short_blocks {
        result.push(block[position]);
      }
    }
  }
  result
}

fn reed_solomon_divisor(degree: usize) -> Vec<u8> {
  let mut result = vec![0u8; degree];
  result[degree - 1] = 1;
  let mut root = 1u8;
  for _ in 0..degree {
    for index in 0..degree {
      result[index] = gf_multiply(result[index], root);
      if index + 1 < degree {
        result[index] ^= result[index + 1];
      }
    }
    root = gf_multiply(root, 0x02);
  }
  result
}

fn reed_solomon_remainder(data: &[u8], divisor: &[u8]) -> Vec<u8> {
  let mut result = vec![0u8; divisor.len()];
  for byte in data {
    let factor = byte ^ result.remove(0);
    result.push(0);
    for (held, coefficient) in result.iter_mut().zip(divisor) {
      *held ^= gf_multiply(*coefficient, factor);
    }
  }
  result
}

/// The product of two elements of GF(2^8) modulo x^8 + x^4 + x^3 + x^2 + 1.
fn gf_multiply(x: u8, y: u8) -> u8 {
  let mut z: u32 = 0;
  for index in (0..8).rev() {
    z = (z << 1) ^ ((z >> 7) * 0x11D);
    z ^= ((u32::from(y) >> index) & 1) * u32::from(x);
  }
  z as u8
}

/// The modules while they are drawn, and which of them are function patterns that no codeword or mask touches.
struct Grid {
  size: usize,
  version: u8,
  modules: Vec<bool>,
  function: Vec<bool>,
}

impl Grid {
  fn new(version: u8) -> Self {
    let size = usize::from(version) * 4 + 17;
    Self {
      size,
      version,
      modules: vec![false; size * size],
      function: vec![false; size * size],
    }
  }

  fn get(&self, x: usize, y: usize) -> bool {
    self.modules[y * self.size + x]
  }

  fn set_function(&mut self, x: usize, y: usize, dark: bool) {
    self.modules[y * self.size + x] = dark;
    self.function[y * self.size + x] = true;
  }

  fn draw_function_patterns(&mut self, level: QrLevel) {
    for index in 0..self.size {
      self.set_function(6, index, index % 2 == 0);
      self.set_function(index, 6, index % 2 == 0);
    }
    let far = self.size as i32 - 4;
    self.draw_finder(3, 3);
    self.draw_finder(far, 3);
    self.draw_finder(3, far);
    let positions = self.alignment_positions();
    let count = positions.len();
    for (i, &x) in positions.iter().enumerate() {
      for (j, &y) in positions.iter().enumerate() {
        // Not where a finder is.
        if (i == 0 && j == 0) || (i == 0 && j == count - 1) || (i == count - 1 && j == 0) {
          continue;
        }
        self.draw_alignment(x, y);
      }
    }
    // Marks the format's modules as function ones; the mask's bits are drawn over them once it is chosen.
    self.draw_format_bits(level, 0);
    self.draw_version();
  }

  fn draw_finder(&mut self, x: i32, y: i32) {
    for dy in -4..=4i32 {
      for dx in -4..=4i32 {
        let distance = dx.abs().max(dy.abs());
        let (xx, yy) = (x + dx, y + dy);
        if (0..self.size as i32).contains(&xx) && (0..self.size as i32).contains(&yy) {
          self.set_function(xx as usize, yy as usize, distance != 2 && distance != 4);
        }
      }
    }
  }

  fn draw_alignment(&mut self, x: usize, y: usize) {
    for dy in -2..=2i32 {
      for dx in -2..=2i32 {
        let dark = dx.abs().max(dy.abs()) != 1;
        self.set_function((x as i32 + dx) as usize, (y as i32 + dy) as usize, dark);
      }
    }
  }

  fn alignment_positions(&self) -> Vec<usize> {
    if self.version == 1 {
      return Vec::new();
    }
    let version = i32::from(self.version);
    let count = version / 7 + 2;
    let step = (version * 8 + count * 3 + 5) / (count * 4 - 4) * 2;
    let mut result: Vec<usize> = (0..count - 1)
      .map(|index| (self.size as i32 - 7 - index * step) as usize)
      .collect();
    result.push(6);
    result.reverse();
    result
  }

  fn draw_format_bits(&mut self, level: QrLevel, mask: u8) {
    let data = u32::from(level.format_bits()) << 3 | u32::from(mask);
    let mut remainder = data;
    for _ in 0..10 {
      remainder = (remainder << 1) ^ ((remainder >> 9) * 0x537);
    }
    let bits = (data << 10 | remainder) ^ 0x5412;
    let bit = |index: usize| (bits >> index) & 1 != 0;
    for index in 0..=5 {
      self.set_function(8, index, bit(index));
    }
    self.set_function(8, 7, bit(6));
    self.set_function(8, 8, bit(7));
    self.set_function(7, 8, bit(8));
    for index in 9..15 {
      self.set_function(14 - index, 8, bit(index));
    }
    let size = self.size;
    for index in 0..8 {
      self.set_function(size - 1 - index, 8, bit(index));
    }
    for index in 8..15 {
      self.set_function(8, size - 15 + index, bit(index));
    }
    // The module that is always dark.
    self.set_function(8, size - 8, true);
  }

  fn draw_version(&mut self) {
    if self.version < 7 {
      return;
    }
    let mut remainder = u32::from(self.version);
    for _ in 0..12 {
      remainder = (remainder << 1) ^ ((remainder >> 11) * 0x1F25);
    }
    let bits = u32::from(self.version) << 12 | remainder;
    for index in 0..18 {
      let dark = (bits >> index) & 1 != 0;
      let (a, b) = (self.size - 11 + index % 3, index / 3);
      self.set_function(a, b, dark);
      self.set_function(b, a, dark);
    }
  }

  /// Lays the codewords in the zigzag of two columns, from the bottom right, up and down in turn.
  fn draw_codewords(&mut self, codewords: &[u8]) {
    let mut index = 0;
    let mut right = self.size as i32 - 1;
    while right >= 1 {
      if right == 6 {
        right = 5;
      }
      for vertical in 0..self.size {
        for column in 0..2 {
          let x = (right - column) as usize;
          let upward = (right + 1) & 2 == 0;
          let y = if upward { self.size - 1 - vertical } else { vertical };
          if !self.function[y * self.size + x] && index < codewords.len() * 8 {
            self.modules[y * self.size + x] = (codewords[index >> 3] >> (7 - (index & 7))) & 1 != 0;
            index += 1;
          }
        }
      }
      right -= 2;
    }
  }

  /// Turns the modules a mask says, but for the function patterns. Twice is none.
  fn apply_mask(&mut self, mask: u8) {
    for y in 0..self.size {
      for x in 0..self.size {
        let turn = match mask {
          0 => (x + y) % 2 == 0,
          1 => y % 2 == 0,
          2 => x % 3 == 0,
          3 => (x + y) % 3 == 0,
          4 => (x / 3 + y / 2) % 2 == 0,
          5 => x * y % 2 + x * y % 3 == 0,
          6 => (x * y % 2 + x * y % 3) % 2 == 0,
          _ => ((x + y) % 2 + x * y % 3) % 2 == 0,
        };
        let at = y * self.size + x;
        self.modules[at] ^= turn && !self.function[at];
      }
    }
  }

  /// How bad the modules are to read: runs, blocks, patterns like a finder's, and the share of dark ones.
  fn penalty(&self) -> i32 {
    let size = self.size;
    let mut result = 0;
    for across in [true, false] {
      for line in 0..size {
        let mut run_color = false;
        let mut run = 0;
        let mut history = [0i32; 7];
        for along in 0..size {
          let dark = if across {
            self.get(along, line)
          } else {
            self.get(line, along)
          };
          if dark == run_color {
            run += 1;
            if run == 5 {
              result += PENALTY_N1;
            } else if run > 5 {
              result += 1;
            }
          } else {
            self.add_history(run, &mut history);
            if !run_color {
              result += count_finder_patterns(&history) * PENALTY_N3;
            }
            run_color = dark;
            run = 1;
          }
        }
        result += self.terminate_and_count(run_color, run, &mut history) * PENALTY_N3;
      }
    }
    for y in 0..size - 1 {
      for x in 0..size - 1 {
        let dark = self.get(x, y);
        if dark == self.get(x + 1, y) && dark == self.get(x, y + 1) && dark == self.get(x + 1, y + 1) {
          result += PENALTY_N2;
        }
      }
    }
    let dark = self.modules.iter().filter(|dark| **dark).count() as i32;
    let total = (size * size) as i32;
    let k = ((dark * 20 - total * 10).abs() + total - 1) / total - 1;
    result + k * PENALTY_N4
  }

  fn add_history(&self, run: i32, history: &mut [i32; 7]) {
    // The light border before the first run.
    let run = if history[0] == 0 { run + self.size as i32 } else { run };
    history.copy_within(0..6, 1);
    history[0] = run;
  }

  fn terminate_and_count(&self, run_color: bool, run: i32, history: &mut [i32; 7]) -> i32 {
    let mut run = run;
    if run_color {
      self.add_history(run, history);
      run = 0;
    }
    // The light border after the last run.
    self.add_history(run + self.size as i32, history);
    count_finder_patterns(history)
  }
}

fn count_finder_patterns(history: &[i32; 7]) -> i32 {
  let n = history[1];
  let core = n > 0 && history[2] == n && history[3] == n * 3 && history[4] == n && history[5] == n;
  i32::from(core && history[0] >= n * 4 && history[6] >= n) + i32::from(core && history[6] >= n * 4 && history[0] >= n)
}
