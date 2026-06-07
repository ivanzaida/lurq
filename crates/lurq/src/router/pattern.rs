use std::sync::Arc;

use super::route_match::Params;

#[derive(Clone, Debug)]
pub struct Pattern {
  segments: Vec<Segment>,
  raw: Arc<str>,
}

#[derive(Clone, Debug)]
enum Segment {
  Static(Arc<str>),
  Param(Arc<str>),
  Wildcard,
  CatchAll(Arc<str>),
}

impl Pattern {
  pub fn new(raw: &str) -> Self {
    let raw_arc: Arc<str> = Arc::from(raw);
    let segments = parse_segments(raw);
    Self { segments, raw: raw_arc }
  }

  pub fn raw(&self) -> &str {
    &self.raw
  }

  pub fn matches(&self, path: &str) -> Option<Params> {
    let path_segments = normalize_segments(path);
    self.match_segments(&path_segments)
  }

  pub(crate) fn matches_segments(&self, segments: &[&str]) -> Option<Params> {
    self.match_segments(segments)
  }

  pub(crate) fn match_prefix(&self, path_segments: &[&str]) -> Option<(Params, usize)> {
    self.match_segments_prefix(path_segments)
  }

  pub fn priority(&self) -> u32 {
    let mut score = 0u32;
    for segment in &self.segments {
      score = score.wrapping_mul(4);
      match segment {
        Segment::Static(_) => score = score.wrapping_add(3),
        Segment::Param(_) => score = score.wrapping_add(2),
        Segment::Wildcard => score = score.wrapping_add(1),
        Segment::CatchAll(_) => {}
      }
    }
    score
  }

  fn match_segments(&self, path_segments: &[&str]) -> Option<Params> {
    let mut params = Params::default();
    let mut pi = 0;

    for segment in &self.segments {
      match segment {
        Segment::Static(s) => {
          if pi >= path_segments.len() || path_segments[pi] != s.as_ref() {
            return None;
          }
          pi += 1;
        }
        Segment::Param(name) => {
          if pi >= path_segments.len() || path_segments[pi].is_empty() {
            return None;
          }
          params.set(name.clone(), Arc::from(path_segments[pi]));
          pi += 1;
        }
        Segment::Wildcard => {
          if pi >= path_segments.len() || path_segments[pi].is_empty() {
            return None;
          }
          pi += 1;
        }
        Segment::CatchAll(name) => {
          if pi >= path_segments.len() {
            return None;
          }
          let rest = path_segments[pi..].join("/");
          if rest.is_empty() {
            return None;
          }
          params.set(name.clone(), Arc::from(rest));
          return Some(params);
        }
      }
    }

    if pi == path_segments.len() { Some(params) } else { None }
  }

  fn match_segments_prefix(&self, path_segments: &[&str]) -> Option<(Params, usize)> {
    let mut params = Params::default();
    let mut pi = 0;

    for segment in &self.segments {
      match segment {
        Segment::Static(s) => {
          if pi >= path_segments.len() || path_segments[pi] != s.as_ref() {
            return None;
          }
          pi += 1;
        }
        Segment::Param(name) => {
          if pi >= path_segments.len() || path_segments[pi].is_empty() {
            return None;
          }
          params.set(name.clone(), Arc::from(path_segments[pi]));
          pi += 1;
        }
        Segment::Wildcard => {
          if pi >= path_segments.len() || path_segments[pi].is_empty() {
            return None;
          }
          pi += 1;
        }
        Segment::CatchAll(name) => {
          if pi >= path_segments.len() {
            return None;
          }
          let rest = path_segments[pi..].join("/");
          if rest.is_empty() {
            return None;
          }
          params.set(name.clone(), Arc::from(rest));
          return Some((params, path_segments.len()));
        }
      }
    }

    Some((params, pi))
  }
}

fn parse_segments(raw: &str) -> Vec<Segment> {
  let trimmed = raw.trim_start_matches('/');
  if trimmed.is_empty() {
    return vec![];
  }

  trimmed
    .split('/')
    .filter(|s| !s.is_empty())
    .map(|s| {
      if let Some(name) = s.strip_prefix("**") {
        Segment::CatchAll(Arc::from(name))
      } else if let Some(name) = s.strip_prefix(':') {
        Segment::Param(Arc::from(name))
      } else if s == "*" {
        Segment::Wildcard
      } else {
        Segment::Static(Arc::from(s))
      }
    })
    .collect()
}

pub(crate) fn normalize_segments(path: &str) -> Vec<&str> {
  let path = path.split(['?', '#']).next().unwrap_or(path);
  let trimmed = path.trim_start_matches('/');
  if trimmed.is_empty() {
    return vec![];
  }
  trimmed.split('/').filter(|s| !s.is_empty()).collect()
}
