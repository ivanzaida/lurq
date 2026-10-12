use std::collections::BTreeMap;

use cosmic_text::{LayoutLine, ShapeLine};

use super::{super::InputConfig, Paragraph, visual_bounds};

pub(in super::super) struct Padding {
  pub(in super::super) edges: BTreeMap<(usize, usize), (f32, f32)>,
}

pub(super) fn apply(shape: &mut ShapeLine, row: &LayoutLine, paragraph: &Paragraph, config: &InputConfig) -> Padding {
  let clusters = shaped_clusters(row, paragraph);
  let mut edges = BTreeMap::new();
  let mut owners = BTreeMap::new();
  for (index, cluster) in clusters.iter().enumerate() {
    let Some(token) = cluster.token else {
      continue;
    };
    let pad = config.styles[token].padding;
    let left = if index == 0 || clusters[index - 1].token != Some(token) {
      pad
    } else {
      0.0
    };
    let right = if index + 1 == clusters.len() || clusters[index + 1].token != Some(token) {
      pad
    } else {
      0.0
    };
    edges.insert(cluster.key, (left, right));
    *owners.entry(cluster.left.2).or_insert(0.0) += left;
    *owners.entry(cluster.right.2).or_insert(0.0) += right;
  }
  for glyph in shape
    .spans
    .iter_mut()
    .flat_map(|span| &mut span.words)
    .flat_map(|word| &mut word.glyphs)
  {
    if let Some(pad) = owners.get(&glyph.metadata) {
      let em = glyph
        .metrics_opt
        .map_or(config.base.font_size, |metrics| metrics.font_size)
        .max(1.0);
      glyph.x_advance += pad / em;
    }
  }
  Padding { edges }
}

pub(super) fn restore_ink(padded: &mut LayoutLine, unpadded: &LayoutLine, padding: &Padding) {
  let old_bounds = visual_bounds(&unpadded.glyphs);
  let new_bounds = visual_bounds(&padded.glyphs);
  let old: BTreeMap<_, _> = unpadded.glyphs.iter().map(|glyph| (glyph.metadata, glyph)).collect();
  for glyph in &mut padded.glyphs {
    let key = (glyph.start, glyph.end);
    if let (Some(old), Some(before), Some(after)) =
      (old.get(&glyph.metadata), old_bounds.get(&key), new_bounds.get(&key))
    {
      let left = padding.edges.get(&key).map_or(0.0, |edges| edges.0);
      let ink = old.x + old.font_size * old.x_offset;
      let target = after.0 + left + ink - before.0;
      glyph.x_offset = (target - glyph.x) / glyph.font_size.max(1.0);
    }
  }
}

struct ShapedCluster {
  key: (usize, usize),
  token: Option<usize>,
  left: (f32, f32, usize),
  right: (f32, f32, usize),
}

fn shaped_clusters(row: &LayoutLine, paragraph: &Paragraph) -> Vec<ShapedCluster> {
  let mut clusters = BTreeMap::new();
  for glyph in &row.glyphs {
    let key = (glyph.start, glyph.end);
    let left = glyph.x.min(glyph.x + glyph.w);
    let right = glyph.x.max(glyph.x + glyph.w);
    let width = glyph.w.abs();
    let entry = clusters.entry(key).or_insert_with(|| ShapedCluster {
      key,
      token: paragraph.styles[glyph.metadata].checked_sub(1),
      left: (left, width, glyph.metadata),
      right: (right, width, glyph.metadata),
    });
    if left < entry.left.0 || (left == entry.left.0 && width > entry.left.1) {
      entry.left = (left, width, glyph.metadata);
    }
    if right > entry.right.0 || (right == entry.right.0 && width > entry.right.1) {
      entry.right = (right, width, glyph.metadata);
    }
  }
  let mut clusters: Vec<_> = clusters.into_values().collect();
  clusters.sort_by(|a, b| a.left.0.total_cmp(&b.left.0).then(a.key.cmp(&b.key)));
  clusters
}
