"""The gallery's own track: 24 seconds of plucked notes over four chords, made of sines so that nothing in it
is anyone else's. Written as a WAV, then encoded by ffmpeg to AAC in an MP4 file (.m4a), which every system
this is to play on decodes.

    python scripts/make-gallery-track.py
"""
import math
import struct
import subprocess
import wave

RATE, SECONDS = 44_100, 24.0
STEPS = [0, 4, 7, 12, 7, 4, 0, 7]  # up and down a chord, a quarter of a second each
ROOTS = [0, -4, -7, -5]  # the chord changes every two seconds

frames = bytearray()
for index in range(int(RATE * SECONDS)):
    t = index / RATE
    root = ROOTS[int(t / 2) % 4]
    pitch = 220 * 2 ** ((STEPS[int(t * 4) % 8] + root) / 12)
    pluck = math.exp(-7 * (t % 0.25))
    bass = 110 * 2 ** (root / 12)
    swell = 0.6 + 0.4 * math.sin(math.tau * t / 8)
    fade = min(1, t / 0.3) * min(1, (SECONDS - t) / 1.5)
    low = 0.12 * swell * math.sin(math.tau * bass * t)
    over = 0.35 * math.sin(2 * math.tau * pitch * t)
    left = fade * (0.22 * pluck * (math.sin(math.tau * pitch * t) + over) + low)
    right = fade * (0.22 * pluck * (math.sin(math.tau * pitch * t + 0.4) + over) + low)
    frames += struct.pack("<hh", int(left * 32767), int(right * 32767))

with wave.open("target/gallery-track.wav", "wb") as out:
    out.setnchannels(2)
    out.setsampwidth(2)
    out.setframerate(RATE)
    out.writeframes(bytes(frames))

subprocess.run(
    ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", "target/gallery-track.wav", "-c:a", "aac", "-b:a", "64k",
     "-movflags", "+faststart", "crates/gallery/assets/media/track.m4a"],
    check=True,
)
print("made crates/gallery/assets/media/track.m4a")
