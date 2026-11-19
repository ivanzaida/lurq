//! What the system is told of the app's sound, and what it tells of it: on iOS, the audio session.
//!
//! - **Its category** ([`AudioCategory`], `App::set_audio_category`): whether the app's sound is heard with the ring
//!   switch on silent. iOS's default (`SoloAmbient`) is not; `Playback` is, as a player of music or of podcasts is.
//! - **An interruption** (a call, another app that takes the sound) pauses a sound where it is. When it ends and iOS
//!   says the sound may go on (`AVAudioSessionInterruptionOptionShouldResume`), a sound it paused plays on by itself,
//!   as Apple's guide for players has it ([`resumes`]).
//!
//! Elsewhere there is no session: a category is kept and does nothing, and nothing is interrupted this way.

use std::sync::atomic::{AtomicU64, Ordering};

/// What the app's sound is to the system: on iOS, the category of its audio session. Elsewhere it changes nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AudioCategory {
  /// The system's default: on iOS a sound of the app's own (a game's, an interface's), silent with the ring switch
  /// on silent, and stopped with the screen locked.
  #[default]
  System,
  /// What a person listens to: music, a podcast, a video. Heard with the ring switch on silent; an app whose
  /// `Info.plist` has `audio` in its `UIBackgroundModes` plays on with the screen locked.
  Playback,
}

/// Tells the system the app's category. On iOS a sound playing goes on under the new one.
pub(crate) fn set_category(category: AudioCategory) {
  #[cfg(target_os = "ios")]
  ios::set_category(category);
  #[cfg(not(target_os = "ios"))]
  let _ = category;
}

/// How many interruptions have ended with the system saying a sound may go on. A sound the system paused plays on
/// when this is more than it was when it was paused. The first call starts listening for them.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
pub(crate) fn resumes() -> u64 {
  #[cfg(target_os = "ios")]
  ios::listen();
  RESUMES.load(Ordering::Acquire)
}

static RESUMES: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "ios")]
mod ios {
  use std::{ptr, sync::Once};

  use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
  };

  use super::{AudioCategory, Ordering, RESUMES};

  #[link(name = "AVFAudio", kind = "framework")]
  unsafe extern "C" {
    static AVAudioSessionCategoryPlayback: *const AnyObject;
    static AVAudioSessionCategorySoloAmbient: *const AnyObject;
    static AVAudioSessionInterruptionNotification: *const AnyObject;
    static AVAudioSessionInterruptionTypeKey: *const AnyObject;
    static AVAudioSessionInterruptionOptionKey: *const AnyObject;
  }

  /// `AVAudioSessionInterruptionTypeEnded`.
  const ENDED: usize = 0;
  /// `AVAudioSessionInterruptionOptionShouldResume`.
  const SHOULD_RESUME: usize = 1;

  fn session() -> Retained<AnyObject> {
    // SAFETY: the shared session, which any thread may ask for.
    unsafe { msg_send![objc2::class!(AVAudioSession), sharedInstance] }
  }

  pub(super) fn set_category(category: AudioCategory) {
    // SAFETY: the session's category is one of AVFAudio's names; an error is not asked for, and leaves it as it was.
    unsafe {
      let name = match category {
        AudioCategory::System => AVAudioSessionCategorySoloAmbient,
        AudioCategory::Playback => AVAudioSessionCategoryPlayback,
      };
      let set: Bool = msg_send![&*session(), setCategory: name, error: ptr::null_mut::<*mut AnyObject>()];
      if !set.as_bool() {
        tracing::warn!("lurq: iOS did not take the audio session's category {category:?}");
      }
    }
  }

  /// Listens, once for the run of the app, for the ends of interruptions.
  pub(super) fn listen() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
      let ended = block2::RcBlock::new(|notification: *mut AnyObject| {
        // SAFETY: the notification iOS posts, whose information holds two numbers under AVFAudio's keys.
        unsafe {
          let Some(notification) = notification.as_ref() else {
            return;
          };
          let info: Option<Retained<AnyObject>> = msg_send![notification, userInfo];
          let Some(info) = info else {
            return;
          };
          let number = |key: *const AnyObject| -> usize {
            let value: Option<Retained<AnyObject>> = msg_send![&*info, objectForKey: key];
            value.map_or(0, |value| msg_send![&*value, unsignedIntegerValue])
          };
          if number(AVAudioSessionInterruptionTypeKey) == ENDED
            && number(AVAudioSessionInterruptionOptionKey) & SHOULD_RESUME != 0
          {
            RESUMES.fetch_add(1, Ordering::AcqRel);
          }
        }
      });
      // SAFETY: the default center, a name of AVFAudio's and a block it calls on the thread that posts.
      unsafe {
        let center: Retained<AnyObject> = msg_send![objc2::class!(NSNotificationCenter), defaultCenter];
        let token: Option<Retained<AnyObject>> = msg_send![
          &*center,
          addObserverForName: AVAudioSessionInterruptionNotification,
          object: ptr::null::<AnyObject>(),
          queue: ptr::null::<AnyObject>(),
          usingBlock: &*ended
        ];
        // It listens for the whole run of the app.
        std::mem::forget(token);
      }
    });
  }
}
