"""Regenerates the weight-probe fonts used by the font-weight regression tests.

Each face draws "a" as a box whose advance width encodes the face's weight
(weight / 2 + 300 units per 1000-unit em), so measured text width identifies the
face that was selected. Names follow the layout of common static font families
(Inter, for one): Regular and Bold use the family name directly, while the other
weights carry the weight in name ID 1 and the family in the typographic name IDs
16/17. The faces are original test data, released with lurq under the MIT license.

Usage: python generate.py   (requires fontTools)
"""

from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

FAMILY = "Lurq Weight Probe"
FACES = {
    "LurqWeightProbe-Regular.ttf": (400, "Regular"),
    "LurqWeightProbe-Medium.ttf": (500, "Medium"),
    "LurqWeightProbe-SemiBold.ttf": (600, "SemiBold"),
    "LurqWeightProbe-Bold.ttf": (700, "Bold"),
}


def box(width, height):
    pen = TTGlyphPen(None)
    pen.moveTo((50, 0))
    pen.lineTo((50, height))
    pen.lineTo((width - 50, height))
    pen.lineTo((width - 50, 0))
    pen.closePath()
    return pen.glyph()


def build(path, weight, style):
    advance = weight // 2 + 300
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder([".notdef", "space", "a", "H"])
    builder.setupCharacterMap({0x20: "space", 0x61: "a", 0x48: "H"})
    builder.setupGlyf({
        ".notdef": box(500, 700),
        "space": TTGlyphPen(None).glyph(),
        "a": box(advance, 500),
        "H": box(advance, 700),
    })
    builder.setupHorizontalMetrics({
        ".notdef": (500, 50),
        "space": (250, 0),
        "a": (advance, 50),
        "H": (advance, 50),
    })
    builder.updateHead(macStyle=1 if style == "Bold" else 0)
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    regular_or_bold = style in ("Regular", "Bold")
    names = {
        "familyName": FAMILY if regular_or_bold else f"{FAMILY} {style}",
        "styleName": style if regular_or_bold else "Regular",
        "uniqueFontIdentifier": f"LurqWeightProbe-{style}",
        "fullName": f"{FAMILY} {style}",
        "psName": f"LurqWeightProbe-{style}",
        "version": "Version 1.000",
    }
    if not regular_or_bold:
        names["typographicFamily"] = FAMILY
        names["typographicSubfamily"] = style
    builder.setupNameTable(names)
    builder.setupOS2(
        usWeightClass=weight,
        fsSelection=0x20 if style == "Bold" else 0x40,
        sTypoAscender=800,
        sTypoDescender=-200,
        sCapHeight=700,
        sxHeight=500,
        usWinAscent=800,
        usWinDescent=200,
    )
    builder.setupPost()
    builder.save(path)


if __name__ == "__main__":
    here = Path(__file__).parent
    for name, (weight, style) in FACES.items():
        build(here / name, weight, style)
