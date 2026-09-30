"""Regenerates the ligature-probe font used by the font-feature regression tests.

A monospaced face (every glyph advances 500 units per 1000-unit em) whose
standard ligature feature (`liga`) replaces "--" with one glyph of a single
cell's advance, drawn from -450 to 450 units around its origin so its ink
covers the preceding cell. Programming fonts such as Geist Mono ship the same
kind of ligature: with `liga` on, " --" loses a cell and the dash is drawn over
the space. The face is original test data, released with lurq under the MIT
license.

Usage: python generate.py   (requires fontTools)
"""

from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

FAMILY = "Lurq Ligature Probe"
ADVANCE = 500
FEATURES = """
languagesystem DFLT dflt;
languagesystem latn dflt;
feature liga {
    sub hyphen hyphen by hyphen_hyphen.liga;
} liga;
"""


def box(left, bottom, right, top):
    pen = TTGlyphPen(None)
    pen.moveTo((left, bottom))
    pen.lineTo((left, top))
    pen.lineTo((right, top))
    pen.lineTo((right, bottom))
    pen.closePath()
    return pen.glyph()


def build(path):
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder([".notdef", "space", "a", "hyphen", "hyphen_hyphen.liga"])
    builder.setupCharacterMap({0x20: "space", 0x61: "a", 0x2D: "hyphen"})
    builder.setupGlyf({
        ".notdef": box(50, 0, 450, 700),
        "space": TTGlyphPen(None).glyph(),
        "a": box(50, 0, 450, 500),
        "hyphen": box(100, 250, 400, 330),
        "hyphen_hyphen.liga": box(-450, 250, 450, 330),
    })
    builder.setupHorizontalMetrics({
        ".notdef": (ADVANCE, 50),
        "space": (ADVANCE, 0),
        "a": (ADVANCE, 50),
        "hyphen": (ADVANCE, 100),
        "hyphen_hyphen.liga": (ADVANCE, -450),
    })
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable({
        "familyName": FAMILY,
        "styleName": "Regular",
        "uniqueFontIdentifier": "LurqLigatureProbe-Regular",
        "fullName": f"{FAMILY} Regular",
        "psName": "LurqLigatureProbe-Regular",
        "version": "Version 1.000",
    })
    builder.setupOS2(
        usWeightClass=400,
        fsSelection=0x40,
        sTypoAscender=800,
        sTypoDescender=-200,
        sCapHeight=700,
        sxHeight=500,
        usWinAscent=800,
        usWinDescent=200,
    )
    builder.setupPost(isFixedPitch=1)
    builder.addOpenTypeFeatures(FEATURES)
    builder.save(path)


if __name__ == "__main__":
    build(Path(__file__).parent / "LurqLigatureProbe-Regular.ttf")
