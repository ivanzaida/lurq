//! A track of captions: what is said in a recording and when, read from a WebVTT or a SubRip file.
//!
//! Nothing here draws. A player given a track shows the cue of the moment it is at ([`VideoPlayer::caption_cues`]);
//! an app that draws its own captions asks the track what is said at a time ([`MediaCaptions::at`]).
//!
//! [`VideoPlayer::caption_cues`]: super::VideoPlayer::caption_cues

use std::sync::Arc;

/// A cue of a track of captions: what is said from `start` to `end`, in seconds from the start of the recording.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaCue {
  /// When it comes, in seconds.
  pub start: f64,
  /// When it goes, in seconds: it is not shown at this moment itself.
  pub end: f64,
  /// What is said, one line of it after another.
  pub text: Arc<str>,
}

impl MediaCue {
  /// A cue that says `text` from `start` to `end`, in seconds.
  pub fn new(start: f64, end: f64, text: &str) -> Self {
    Self {
      start,
      end,
      text: Arc::from(text),
    }
  }
}

/// A track of captions: its cues, in the order they start. It is cheap to clone: a player is given one at every
/// render.
///
/// ```
/// use lurq::ui::MediaCaptions;
///
/// let track = MediaCaptions::parse(
///   "WEBVTT\n\n\
///    00:01.000 --> 00:04.000\nThe bar fades out while the video plays.\n\n\
///    00:04.000 --> 00:06.500\nMove the pointer and it comes back.\n",
/// );
/// assert_eq!(track.cues().len(), 2);
/// assert_eq!(track.at(4.5).as_deref(), Some("Move the pointer and it comes back."));
/// assert_eq!(track.at(0.5), None);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaCaptions {
  cues: Arc<[MediaCue]>,
}

impl MediaCaptions {
  /// A track of `cues`, put in the order they start. A cue that says nothing, or that does not end after it
  /// starts, is left out.
  pub fn new(cues: impl IntoIterator<Item = MediaCue>) -> Self {
    let mut cues: Vec<MediaCue> = cues
      .into_iter()
      .filter(|cue| cue.start.is_finite() && cue.end > cue.start && !cue.text.trim().is_empty())
      .collect();
    cues.sort_by(|one, other| one.start.total_cmp(&other.start));
    Self { cues: cues.into() }
  }

  /// Reads a track from the text of a WebVTT (`.vtt`) or a SubRip (`.srt`) file: the two differ in their header
  /// and in the mark before the thousandths of a second, and both are read here.
  ///
  /// A block is a cue where one of its lines has `-->` between two times; what is before that line (a number, a
  /// name) and what follows the times on it (WebVTT's settings of where a cue stands) is passed over, and so are
  /// WebVTT's `NOTE`, `STYLE` and `REGION` blocks. The lines after it are what is said, with their tags
  /// (`<i>`, `<v Ada>`, `{\an8}`) taken out and `&amp;`, `&lt;`, `&gt;` and `&nbsp;` read as what they stand for.
  /// What cannot be read is left out, and nothing is said of it: a track with one bad block is the rest of it.
  pub fn parse(text: &str) -> Self {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut cues = Vec::new();
    for block in text.split("\n\n") {
      let mut lines = block.lines().skip_while(|line| line.trim().is_empty()).peekable();
      match lines.peek() {
        Some(first)
          if ["WEBVTT", "NOTE", "STYLE", "REGION"]
            .iter()
            .any(|word| first.starts_with(word))
            && !first.contains("-->") => {}
        Some(_) => {
          // A number or a name may stand before the times.
          let Some((start, end)) = lines.by_ref().take(2).find_map(times) else {
            continue;
          };
          let said: Vec<String> = lines.map(spoken).filter(|line| !line.is_empty()).collect();
          cues.push(MediaCue {
            start,
            end,
            text: Arc::from(said.join("\n")),
          });
        }
        None => {}
      }
    }
    Self::new(cues)
  }

  /// Its cues, in the order they start.
  pub fn cues(&self) -> &[MediaCue] {
    &self.cues
  }

  pub fn is_empty(&self) -> bool {
    self.cues.is_empty()
  }

  /// What is said at `time`, in seconds: the cue that holds then, or each of them on lines of its own where more
  /// than one does, in the order they started. `None` between two cues.
  pub fn at(&self, time: f64) -> Option<Arc<str>> {
    // The cues that have started by `time` are the first of them: they are in the order they start.
    let started = self.cues.partition_point(|cue| cue.start <= time);
    let mut held = self.cues[..started].iter().filter(|cue| time < cue.end);
    let first = held.next()?;
    match held.next() {
      None => Some(first.text.clone()),
      Some(second) => {
        let mut said = format!("{}\n{}", first.text, second.text);
        for cue in held {
          said.push('\n');
          said.push_str(&cue.text);
        }
        Some(Arc::from(said))
      }
    }
  }
}

/// The two times of a line of a cue, in seconds: `00:01.000 --> 00:04.000`, with what follows them passed over.
fn times(line: &str) -> Option<(f64, f64)> {
  let (start, rest) = line.split_once("-->")?;
  let end = rest.split_whitespace().next()?;
  Some((seconds(start.trim())?, seconds(end)?))
}

/// A time in seconds: `1:02:03.500`, `02:03.500` or `02:03,500`, the hours left out where there are none.
fn seconds(time: &str) -> Option<f64> {
  let time = time.replace(',', ".");
  let mut parts = time.rsplit(':');
  let seconds: f64 = parts.next()?.parse().ok()?;
  let minutes: f64 = parts.next()?.parse().ok()?;
  let hours: f64 = match parts.next() {
    Some(hours) => hours.parse().ok()?,
    None => 0.0,
  };
  let whole = parts.next().is_none() && [seconds, minutes, hours].iter().all(|part| *part >= 0.0);
  whole.then_some(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// A line of a cue as it is said: its tags (`<i>`, `<v Ada>`, `<00:01.000>`, `{\an8}`) taken out, and what stands
/// for a character read as it.
fn spoken(line: &str) -> String {
  let mut said = String::with_capacity(line.len());
  let mut closes = None;
  for character in line.chars() {
    match (closes, character) {
      (Some(close), character) if character == close => closes = None,
      (Some(_), _) => {}
      (None, '<') => closes = Some('>'),
      (None, '{') => closes = Some('}'),
      (None, character) => said.push(character),
    }
  }
  [
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&nbsp;", " "),
    ("&lrm;", ""),
    ("&rlm;", ""),
    ("&amp;", "&"),
  ]
  .iter()
  .fold(said, |said, (name, is)| said.replace(name, is))
  .trim()
  .to_owned()
}
