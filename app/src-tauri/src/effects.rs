//! Effects games draw around their letters (outline, halo, drop shadow),
//! measured on a line's pixels so the translation can be drawn with the same.
//!
//! The overlay now hides the original line with the game's own background
//! (the letters and their shadow are taken out), so a translation drawn
//! without the effect looks flatter than the game. Here:
//!  1. letter pixels: very close to the line's text color;
//!  2. around them, in 8 directions at 1..=3 pixels, each pixel is compared
//!     with the local background (the pixel 5 px away in the same direction:
//!     a line can cross a dark box and a bright map). A mix of the text and
//!     that background is just the letter's soft edge and doesn't count; what
//!     strays from that mix (a shadow darker than the background) does;
//!  3. the same all around = an outline (or halo); much more on one side
//!     than on the opposite one = a drop shadow towards that side.

use serde::Serialize;

/// Directions around a letter pixel: (dx, dy).
const DIRECTIONS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
const REACH: usize = 3; // pixels looked at around the letters
const LOCAL: i32 = 5; // the local background is this far from the letter, in the same direction
const MIN_LETTER_PIXELS: usize = 12; // fewer letter pixels: nothing to measure
const STRONG: f32 = 40.0; // how far from the text/background mix (RGB distance) counts as an effect
const ONE_SIDED: f32 = 0.45; // the opposite side has less than this share: a shadow, not an outline

/// How the game decorates the letters of a line. Sizes in window pixels.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEffect {
    pub kind: &'static str, // "outline" (all around: outline or halo) | "shadow" (to one side)
    pub color: String,      // "#rrggbb"
    pub size: f32,          // outline: how far it reaches; shadow: its blur
    pub dx: i32,            // shadow offset (0 for an outline)
    pub dy: i32,
}

/// RGB distance between two 0xRRGGBB colors.
fn distance(a: u32, b: u32) -> f32 {
    let d = |s: u32| ((a >> s) & 0xFF) as f32 - ((b >> s) & 0xFF) as f32;
    (d(16).powi(2) + d(8).powi(2) + d(0).powi(2)).sqrt()
}

/// How far `c` is from every mix of `bg` and `text` (the segment between the
/// two colors): ~0 for a letter's anti-aliased edge, large for a shadow or an
/// outline of another color.
fn off_mix(c: u32, bg: u32, text: u32) -> f32 {
    let v = |x: u32| [((x >> 16) & 0xFF) as f32, ((x >> 8) & 0xFF) as f32, (x & 0xFF) as f32];
    let (c, b, t) = (v(c), v(bg), v(text));
    let axis = [t[0] - b[0], t[1] - b[1], t[2] - b[2]];
    let len2 = axis.iter().map(|a| a * a).sum::<f32>().max(1.0);
    let along = ((0..3).map(|i| (c[i] - b[i]) * axis[i]).sum::<f32>() / len2).clamp(0.0, 1.0);
    (0..3).map(|i| (c[i] - (b[i] + axis[i] * along)).powi(2)).sum::<f32>().sqrt()
}

/// Measures the effect around the letters of a line. `pixel(x, y)` gives the
/// color at a point of the picture (None outside it); `line` is the line's
/// box (x, y, width, height) in the same coordinates. None when the letters
/// have no visible effect, or there is too little to tell.
pub fn measure(pixel: impl Fn(i32, i32) -> Option<u32>, line: (i32, i32, i32, i32), text: u32, background: u32) -> Option<TextEffect> {
    let (lx, ly, lw, lh) = line;
    let contrast = distance(text, background);
    if lw <= 0 || lh <= 0 || contrast < STRONG {
        return None;
    }
    // Letter pixels: clearly the text color (anti-aliased edges left out).
    let is_letter = |x: i32, y: i32| pixel(x, y).is_some_and(|c| distance(c, text) < contrast * 0.35);
    let near_text = |c: u32| distance(c, text) < distance(c, background);

    // For each direction and distance: summed difference from the background,
    // summed colors and how many pixels.
    let mut diff = [[0f32; REACH]; 8];
    let mut count = [[0u32; REACH]; 8];
    let mut color = [[0f32; 3]; 8]; // colors of the effect's own pixels (clearly off the background)
    let mut colored = [0u32; 8];
    let mut letters = 0usize;
    for y in ly..ly + lh {
        for x in lx..lx + lw {
            if !is_letter(x, y) {
                continue;
            }
            letters += 1;
            for (k, (dx, dy)) in DIRECTIONS.iter().enumerate() {
                // The background right there, past any effect (none: not measured this way).
                let Some(local) = pixel(x + dx * LOCAL, y + dy * LOCAL).filter(|&c| !near_text(c)) else {
                    continue;
                };
                for r in 1..=REACH as i32 {
                    let (px, py) = (x + dx * r, y + dy * r);
                    let Some(c) = pixel(px, py) else { break };
                    if is_letter(px, py) {
                        break; // the next letter: not the effect
                    }
                    let i = (r - 1) as usize;
                    let off = off_mix(c, local, text);
                    diff[k][i] += off;
                    count[k][i] += 1;
                    if r == 1 && off >= STRONG {
                        colored[k] += 1;
                        color[k][0] += ((c >> 16) & 0xFF) as f32;
                        color[k][1] += ((c >> 8) & 0xFF) as f32;
                        color[k][2] += (c & 0xFF) as f32;
                    }
                }
            }
        }
    }
    if letters < MIN_LETTER_PIXELS {
        return None;
    }
    let mean = |k: usize, r: usize| if count[k][r] > 0 { diff[k][r] / count[k][r] as f32 } else { 0.0 };
    let side: Vec<f32> = (0..8).map(|k| mean(k, 0)).collect(); // how strong right next to the letters, per direction
    let strongest = (0..8).max_by(|&a, &b| side[a].total_cmp(&side[b]))?;
    if side[strongest] < STRONG {
        return None; // the letters sit right on the background
    }
    let opposite = (strongest + 4) % 8;
    let rgb_of = |k: usize| {
        let n = colored[k].max(1) as f32;
        let c = color[k];
        (c[0] / n).round() as u32 * 65536 + (c[1] / n).round() as u32 * 256 + (c[2] / n).round() as u32
    };

    if side[opposite] < side[strongest] * ONE_SIDED {
        // A drop shadow: as far as it stays strong in that direction.
        let reach = (0..REACH).take_while(|&r| mean(strongest, r) >= STRONG).count().max(1) as i32;
        let (dx, dy) = DIRECTIONS[strongest];
        return Some(TextEffect {
            kind: "shadow",
            color: format!("#{:06x}", rgb_of(strongest)),
            size: if reach > 1 { 1.0 } else { 0.5 },
            dx: dx * reach,
            dy: dy * reach,
        });
    }

    // All around: an outline (or a halo) as wide as it stays strong on average.
    let around = |r: usize| (0..8).map(|k| mean(k, r)).sum::<f32>() / 8.0;
    let width = (0..REACH).take_while(|&r| around(r) >= STRONG * 0.6).count().max(1);
    let darkest = (0..8).max_by(|&a, &b| side[a].total_cmp(&side[b]))?;
    Some(TextEffect {
        kind: "outline",
        color: format!("#{:06x}", rgb_of(darkest)),
        size: width as f32,
        dx: 0,
        dy: 0,
    })
}

/// The effect of a whole text from its lines' ones: the most common kind,
/// with the median size, when at least a third of the lines show one. None
/// otherwise (then each line keeps what was measured on it).
pub fn shared(lines: &[Option<TextEffect>]) -> Option<TextEffect> {
    let found: Vec<&TextEffect> = lines.iter().flatten().collect();
    if found.is_empty() || found.len() * 3 < lines.len() {
        return None;
    }
    let outlines = found.iter().filter(|e| e.kind == "outline").count();
    let kind = if outlines * 2 >= found.len() { "outline" } else { "shadow" };
    let mut same: Vec<&TextEffect> = found.into_iter().filter(|e| e.kind == kind).collect();
    same.sort_by(|a, b| a.size.total_cmp(&b.size));
    Some(same[same.len() / 2].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BG: u32 = 0x506070;
    const TEXT: u32 = 0xFFFFFF;
    const DARK: u32 = 0x000000;

    /// A 60 x 30 picture: background, "letters" (vertical bars 3 px wide at
    /// x = 10, 20, 30, 40, from y = 10 to 20), and whatever `paint` adds.
    fn picture(paint: impl Fn(i32, i32, &mut u32)) -> Vec<u32> {
        let mut out = vec![BG; 60 * 30];
        for y in 0..30 {
            for x in 0..60 {
                paint(x, y, &mut out[(y * 60 + x) as usize]);
            }
        }
        out
    }
    fn is_bar(x: i32, y: i32) -> bool {
        (10..20).contains(&y) && [10, 20, 30, 40].iter().any(|&b| (b..b + 3).contains(&x))
    }
    fn run(pixels: &[u32]) -> Option<TextEffect> {
        let pixel = |x: i32, y: i32| ((0..60).contains(&x) && (0..30).contains(&y)).then(|| pixels[(y * 60 + x) as usize]);
        measure(pixel, (5, 5, 50, 20), TEXT, BG)
    }

    #[test]
    fn plain_letters_have_no_effect() {
        let pixels = picture(|x, y, c| {
            if is_bar(x, y) {
                *c = TEXT
            }
        });
        assert_eq!(run(&pixels), None);
    }

    #[test]
    fn a_dark_ring_is_an_outline() {
        let pixels = picture(|x, y, c| {
            let near = (-1..=1).any(|dy| (-1..=1).any(|dx| is_bar(x + dx, y + dy)));
            if is_bar(x, y) {
                *c = TEXT
            } else if near {
                *c = DARK
            }
        });
        let effect = run(&pixels).expect("an outline");
        assert_eq!(effect.kind, "outline");
        assert_eq!(effect.color, "#000000");
        assert_eq!(effect.size, 1.0);
    }

    #[test]
    fn a_copy_down_right_is_a_drop_shadow() {
        let pixels = picture(|x, y, c| {
            if is_bar(x, y) {
                *c = TEXT
            } else if is_bar(x - 2, y - 2) {
                *c = DARK
            }
        });
        let effect = run(&pixels).expect("a shadow");
        assert_eq!(effect.kind, "shadow");
        assert_eq!((effect.dx, effect.dy), (2, 2), "2 px down and to the right: {effect:?}");
        assert_eq!(effect.color, "#000000");
    }

    #[test]
    fn a_text_shares_its_most_common_effect() {
        let halo = |size: f32| Some(TextEffect { kind: "outline", color: "#000000".into(), size, dx: 0, dy: 0 });
        // The Wesnoth help page: the halo shows over the map, not over the dark box.
        let lines = [None, halo(3.0), halo(3.0), halo(2.0), halo(3.0), None];
        assert_eq!(shared(&lines).map(|e| e.size), Some(3.0));
        assert_eq!(shared(&[None, None, halo(2.0), None, None, None, None]), None); // too few to tell
        assert_eq!(shared(&[]), None);
    }

    #[test]
    fn low_contrast_text_is_left_alone() {
        let pixels = picture(|x, y, c| {
            if is_bar(x, y) {
                *c = 0x58687A
            }
        });
        let pixel = |x: i32, y: i32| ((0..60).contains(&x) && (0..30).contains(&y)).then(|| pixels[(y * 60 + x) as usize]);
        assert_eq!(measure(pixel, (5, 5, 50, 20), 0x58687A, BG), None);
    }
}
