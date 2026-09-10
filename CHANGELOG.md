# Changelog

## 0.18.1

- Masked inputs render U+2022 (`•`) by default (was `*`). Custom mask characters and unmasking remain supported.
- Documented font fallback for mask glyphs and added regression coverage for bullet rendering, width, caret placement, and selection deletion with ASCII and multibyte values.
