# Changelog

## v0.2.0 — 2026-11-06

Translations now take the original text's place much more faithfully, tested
hard on Hades II (small icons, colorful animated backgrounds, two-color lines).

### Translation in place
- The original lines are hidden with the game's own background: the core takes
  the text out of the picture, and only where the letters were is covered, so
  the live, animated game shows everywhere else.
- Translations keep the game's look: letter case per line (small capitals),
  outline, halo or drop shadow around the letters, stroke weight with variable
  fonts, and words the game writes in another color ("Grasp" in green, names in
  white on a purple line).
- Paragraphs: titles, list items and wider gaps split blocks; a paragraph that
  grows in translation pushes the ones below it down at the game's spacing,
  while text at the foot of the box stays put, and nothing leaves the marked area.
- Lone titles stay on one line; lone lines follow their neighbors' alignment.

### Icons inside the text
- Buttons, keys, notes, coins and clocks become placeholders (`{1}`) that the
  translation keeps, drawn back at their size in the game.
- Found when OCR reads them as letters ("C" for a clock, "2746" for 274 and
  coins), skips them, or splits the line around them.
- Text OCR leaves out entirely ("· ? ? ?") moves along with the translation as a
  picture instead of being covered.

### Quality
- Per-game glossary, with terms picked right over the game (Ctrl+Shift+G or the
  toolbar's book button).
- Help: the guided tour now covers the glossary, and the in-game toolbar has a
  "?" button that explains each of its buttons (pressing one while the guide is
  open explains it instead of running it).
- Versioned per-game profiles, with automatic migration of old data.
- History groups original and translation, joins wrapped lines, and shows tags
  (cache, glossary, re-reading) and how long each stage took.
- Pluggable OCR engines (Windows OCR by default; PaddleOCR planned as an option).
- The quick translator (Google) pauses for 10 minutes when it answers "too many
  requests", instead of failing on every line.

### Known issues
- **Whole-window reading is experimental.** On busy screens translations may land
  out of place, cut off or overlapping; the app now says so. Marking the areas
  where text shows up gives far better results.
- The translator may still pick its own names for proper nouns; add them to the
  game's glossary to fix them.
