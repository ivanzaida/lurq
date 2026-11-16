//! A track of captions read from a WebVTT or a SubRip file, and what it says at a time.

use lurq::ui::{MediaCaptions, MediaCue};

const VTT: &str = "WEBVTT - A talk\n\
\n\
NOTE\n\
This block is a note, with a line of its own.\n\
\n\
STYLE\n\
::cue { color: yellow }\n\
\n\
intro\n\
00:00:01.000 --> 00:00:04.000 line:80% align:center\n\
<v Ada>The bar fades out</v>\n\
while the video <i>plays</i>.\n\
\n\
00:04.500 --> 00:06.250\n\
Tom &amp; Ada: 2 &lt; 3\n\
\n\
1:00:00.000 --> 1:00:02.000\n\
An hour in.\n";

const SRT: &str = "1\r\n\
00:00:01,000 --> 00:00:04,000\r\n\
{\\an8}The bar fades out\r\n\
while the video plays.\r\n\
\r\n\
2\r\n\
00:00:04,500 --> 00:00:06,250\r\n\
<b>Move</b> the pointer.\r\n";

#[test]
fn a_webvtt_file_is_its_cues_without_its_notes_its_styles_and_its_tags() {
  let track = MediaCaptions::parse(VTT);
  assert_eq!(
    track.cues(),
    [
      MediaCue::new(1.0, 4.0, "The bar fades out\nwhile the video plays."),
      MediaCue::new(4.5, 6.25, "Tom & Ada: 2 < 3"),
      MediaCue::new(3600.0, 3602.0, "An hour in."),
    ]
  );
}

#[test]
fn a_subrip_file_is_read_as_a_webvtt_one_is() {
  let track = MediaCaptions::parse(SRT);
  assert_eq!(
    track.cues(),
    [
      MediaCue::new(1.0, 4.0, "The bar fades out\nwhile the video plays."),
      MediaCue::new(4.5, 6.25, "Move the pointer."),
    ]
  );
  // The mark of the byte order a file may start with is passed over.
  assert_eq!(MediaCaptions::parse(&format!("\u{feff}{SRT}")), track);
}

#[test]
fn a_cue_is_said_from_its_start_to_its_end_and_nothing_between_two() {
  let track = MediaCaptions::parse(VTT);
  assert_eq!(track.at(0.99), None, "before the first");
  assert_eq!(
    track.at(1.0).as_deref(),
    Some("The bar fades out\nwhile the video plays."),
    "from its start"
  );
  assert_eq!(
    track.at(3.999).as_deref(),
    Some("The bar fades out\nwhile the video plays.")
  );
  assert_eq!(track.at(4.0), None, "not at its end");
  assert_eq!(track.at(5.0).as_deref(), Some("Tom & Ada: 2 < 3"));
  assert_eq!(track.at(100.0), None);
  assert_eq!(track.at(3601.0).as_deref(), Some("An hour in."));
  assert_eq!(MediaCaptions::default().at(1.0), None);
}

#[test]
fn cues_that_hold_together_are_said_on_lines_of_their_own_in_the_order_they_started() {
  let track = MediaCaptions::new([
    MediaCue::new(5.0, 9.0, "Second."),
    MediaCue::new(1.0, 8.0, "First."),
    MediaCue::new(6.0, 7.0, "Third."),
  ]);
  assert_eq!(track.cues()[0].text.as_ref(), "First.", "in the order they start");
  assert_eq!(track.at(2.0).as_deref(), Some("First."));
  assert_eq!(track.at(5.5).as_deref(), Some("First.\nSecond."));
  assert_eq!(track.at(6.5).as_deref(), Some("First.\nSecond.\nThird."));
  assert_eq!(track.at(8.5).as_deref(), Some("Second."));
}

#[test]
fn what_cannot_be_read_is_left_out_and_the_rest_is_the_track() {
  let track = MediaCaptions::parse(
    "WEBVTT\n\n\
     not a time --> at all\nLost.\n\n\
     00:05.000 --> 00:03.000\nEnds before it starts.\n\n\
     00:07.000 --> 00:08.000\n\n\
     00:09.000 --> 00:10.000\nKept.\n\n\
     a line with no times\n",
  );
  assert_eq!(track.cues(), [MediaCue::new(9.0, 10.0, "Kept.")]);
  assert!(MediaCaptions::parse("").is_empty());
  assert!(MediaCaptions::parse("WEBVTT").is_empty());
}
