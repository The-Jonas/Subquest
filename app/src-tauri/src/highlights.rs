//! Words the game writes in another color than the rest of their line: names
//! and terms ("Mostly used to raise [Grasp] in green", "Grown in the
//! [Crossroads] in white" on a purple line).
//!
//! Each word's color is measured on the picture. The line's color is the one
//! most of its letters have (the average of all of them gave Hades' white and
//! green line a mint color that is neither). Runs of words in another color
//! are wrapped in tags, `<c1>Grasp</c1>`, before the text goes to the
//! translator, which keeps them around the words that translate them
//! ("<c1>Compreensão</c1>"); the overlay draws each tag in its color.

/// Squared RGB distance between two 0xRRGGBB colors.
fn gap2(a: u32, b: u32) -> i32 {
    let d = |s: u32| ((a >> s) & 0xFF) as i32 - ((b >> s) & 0xFF) as i32;
    d(16) * d(16) + d(8) * d(8) + d(0) * d(0)
}

const SAME: i32 = 50; // colors closer than this (RGB distance) are the same color
const CLEAR: i32 = 45; // a word's ink must stand this far from the background to have a color

/// Color of a word's letters: the average of the pixels at least half as far
/// from the background as the farthest one (the letters' cores, not their soft
/// edges). None when nothing stands out (too faint to say).
pub fn word_color(width: usize, height: usize, pixel: impl Fn(usize, usize) -> u32, background: u32) -> Option<u32> {
    let mut far = 0;
    for y in 0..height {
        for x in 0..width {
            far = far.max(gap2(pixel(x, y), background));
        }
    }
    if far < CLEAR * CLEAR {
        return None;
    }
    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
    for y in 0..height {
        for x in 0..width {
            let c = pixel(x, y);
            if gap2(c, background) * 4 >= far {
                r += u64::from((c >> 16) & 0xFF);
                g += u64::from((c >> 8) & 0xFF);
                b += u64::from(c & 0xFF);
                n += 1;
            }
        }
    }
    (n > 0).then(|| ((r / n) as u32) << 16 | ((g / n) as u32) << 8 | (b / n) as u32)
}

/// A line written again with tags around the words in another color.
#[derive(Debug, PartialEq)]
pub struct Marked {
    pub text: String, // the words, tagged: "Mostly used to raise {1} <c1>Grasp,</c1> to"
    pub base: u32,    // the color most of the line's letters have
}

/// Tags the runs of words (text, measured color) that differ from the line's
/// main color. `colors` collects the tags' colors for the whole reading (tag
/// N is colors[N-1]), so tags stay unique across lines. Words with no color
/// (pictures "{1}", marks too faint) take no side: they end a run. None when
/// the whole line is one color.
///
/// `prefer`: the reading's main color (see `main_color`). When the line has
/// it, it is the line's color, so lines that make one paragraph agree (the
/// overlay draws a paragraph in its first line's color, the tags in theirs).
pub fn mark(words: &[(String, Option<u32>)], prefer: Option<u32>, colors: &mut Vec<u32>) -> Option<Marked> {
    // Groups of alike colors, weighted by letters.
    let mut groups: Vec<(u32, usize)> = Vec::new();
    let group_of = |groups: &Vec<(u32, usize)>, c: u32| groups.iter().position(|g| gap2(g.0, c) < SAME * SAME);
    for (text, color) in words {
        let Some(c) = *color else { continue };
        let letters = text.chars().filter(|ch| ch.is_alphanumeric()).count();
        match group_of(&groups, c) {
            Some(i) => groups[i].1 += letters,
            None => groups.push((c, letters)),
        }
    }
    if groups.len() < 2 {
        return None;
    }
    let base = match prefer.and_then(|p| group_of(&groups, p)) {
        Some(i) => groups[i].0,
        None => groups.iter().max_by_key(|g| g.1)?.0,
    };
    let mut out = String::new();
    let mut open: Option<(usize, u32)> = None; // (tag number, its color)
    for (text, color) in words {
        let other = color.filter(|&c| gap2(c, base) >= SAME * SAME && text.chars().any(char::is_alphanumeric));
        let same_run = matches!((open, other), (Some((_, a)), Some(b)) if gap2(a, b) < SAME * SAME);
        if !same_run {
            if let Some((n, _)) = open.take() {
                out.push_str(&format!("</c{n}>"));
            }
        }
        if !out.is_empty() {
            out.push(' ');
        }
        if let (None, Some(c)) = (open, other) {
            colors.push(c);
            open = Some((colors.len(), c));
            out.push_str(&format!("<c{}>", colors.len()));
        }
        out.push_str(text);
    }
    if let Some((n, _)) = open {
        out.push_str(&format!("</c{n}>"));
    }
    if colors.is_empty() || !out.contains("<c") {
        return None;
    }
    Some(Marked { text: out, base })
}

/// The color most letters of a reading have, over all its lines.
pub fn main_color(lines: &[Vec<(String, Option<u32>)>]) -> Option<u32> {
    let mut groups: Vec<(u32, usize)> = Vec::new();
    for (text, color) in lines.iter().flatten() {
        let Some(c) = *color else { continue };
        let letters = text.chars().filter(|ch| ch.is_alphanumeric()).count();
        match groups.iter().position(|g| gap2(g.0, c) < SAME * SAME) {
            Some(i) => groups[i].1 += letters,
            None => groups.push((c, letters)),
        }
    }
    groups.iter().max_by_key(|g| g.1).map(|g| g.0)
}

/// The text without its color tags (for translators that would mangle them).
pub fn strip(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let end = tail.find('>').map(|e| e + 1);
        let tag = end.map(|e| &tail[..e]);
        let is_tag = tag.is_some_and(|t| {
            let inner = t.trim_start_matches('<').trim_start_matches('/').trim_end_matches('>');
            inner.len() > 1 && inner.starts_with('c') && inner[1..].chars().all(|c| c.is_ascii_digit())
        });
        if is_tag {
            rest = &tail[end.unwrap()..];
        } else {
            out.push('<');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: u32 = 0xE6E6E6;
    const PURPLE: u32 = 0x8A7299;
    const GREEN: u32 = 0x3EE0A0;

    fn w(text: &str, color: Option<u32>) -> (String, Option<u32>) {
        (text.to_string(), color)
    }

    #[test]
    fn tags_a_green_term_in_a_white_line() {
        let mut colors = Vec::new();
        let words = [w("Mostly", Some(WHITE)), w("used", Some(WHITE)), w("to", Some(WHITE)), w("raise", Some(WHITE)), w("{1}", None), w("Grasp,", Some(GREEN)), w("to", Some(WHITE))];
        let m = mark(&words, None, &mut colors).unwrap();
        assert_eq!(m.text, "Mostly used to raise {1} <c1>Grasp,</c1> to");
        assert_eq!(m.base, WHITE);
        assert_eq!(colors, vec![GREEN]);
    }

    #[test]
    fn the_line_color_is_the_one_most_letters_have() {
        let mut colors = Vec::new();
        let words = [w("·", Some(PURPLE)), w("Grown", Some(PURPLE)), w("in", Some(PURPLE)), w("the", Some(PURPLE)), w("Crossroads", Some(WHITE)), w("using", Some(PURPLE))];
        let m = mark(&words, None, &mut colors).unwrap();
        assert_eq!(m.text, "· Grown in the <c1>Crossroads</c1> using");
        assert_eq!(m.base, PURPLE);
    }

    #[test]
    fn neighbors_in_the_same_color_share_a_tag_and_tags_count_across_lines() {
        let mut colors = vec![GREEN]; // a tag from the line before
        let words = [w("activate", Some(WHITE)), w("more", Some(WHITE)), w("Arcana", Some(0xF0F0F0 - 0x101010)), w("Cards.", Some(0xFAFAFA))];
        assert_eq!(mark(&words, None, &mut colors), None); // all white-ish: one color

        let words = [w("on", Some(PURPLE)), w("Lost", Some(WHITE)), w("Shades", Some(WHITE)), w("in", Some(PURPLE)), w("some", Some(PURPLE)), w("Locations", Some(WHITE))];
        // Mostly white letters, but the reading is mostly purple: purple is the line's color.
        let m = mark(&words, Some(PURPLE), &mut colors).unwrap();
        assert_eq!(m.text, "on <c2>Lost Shades</c2> in some <c3>Locations</c3>");
        assert_eq!(m.base, PURPLE);
        let mut fresh = Vec::new();
        assert_eq!(mark(&words, None, &mut fresh).unwrap().base, WHITE);
        assert_eq!(main_color(&[words.to_vec(), vec![w("Psyche", Some(WHITE))]]), Some(WHITE));
    }

    #[test]
    fn word_color_reads_the_letters_cores() {
        // A word: white strokes with soft gray edges on a dark background.
        let px = |x: usize, _y: usize| match x % 4 {
            0 => 0xF0F0F0,
            1 => 0x606060,
            _ => 0x101010,
        };
        assert_eq!(word_color(20, 10, px, 0x101010), Some(0xF0F0F0));
        assert_eq!(word_color(20, 10, |_, _| 0x141414, 0x101010), None);
    }

    #[test]
    fn strips_only_color_tags() {
        assert_eq!(strip("raise {1} <c1>Grasp</c1>, a <b> c"), "raise {1} Grasp, a <b> c");
    }
}
