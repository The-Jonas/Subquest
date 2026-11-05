//! The look of a typeface, measured on a picture of a line of text, in a way
//! that doesn't depend on what the line says nor on its size: so a menu item
//! and a long sentence in the same font measure alike, and two fonts of the
//! same color don't.
//!
//! The letters are separated from the background by color (the line's letter
//! and background colors, which the core already measures), then four numbers
//! describe them, all relative to the height of the letters' body:
//!   - stroke:   how thick the vertical strokes are (bold vs regular)
//!   - contrast: thick vertical strokes vs thin horizontal ones (serif-like
//!               fonts are high, plain sans fonts are near 1)
//!   - density:  how much of the line's box is ink (wide/heavy vs light/narrow)
//!   - slant:    the lean of the letters (italic vs upright)
//!
//! This doesn't name a font, it only tells whether two lines look written in
//! the same one, which is what text styles need.

use serde::{Deserialize, Serialize};

const MIN_BODY: usize = 8; // letters shorter than this (px) are too small to measure
const MIN_INK: usize = 60; // fewer ink pixels than this: not enough to judge
const SHEARS: [f32; 13] = [-0.3, -0.25, -0.2, -0.15, -0.1, -0.05, 0.0, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3];

/// Size-free description of how a line's letters are drawn.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Shape {
    pub stroke: f32,
    pub contrast: f32,
    pub density: f32,
    pub slant: f32,
}

/// Distance between two 0xRRGGBB colors, squared (enough to compare).
fn color_gap(a: u32, b: u32) -> i32 {
    let d = |shift: u32| ((a >> shift) & 0xFF) as i32 - ((b >> shift) & 0xFF) as i32;
    d(16) * d(16) + d(8) * d(8) + d(0) * d(0)
}

/// Median of a list of run lengths (0 when empty).
fn median(mut values: Vec<usize>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_unstable();
    values[values.len() / 2] as f32
}

/// Lengths of the runs of `true` along a line of cells.
fn runs(cells: impl Iterator<Item = bool>, out: &mut Vec<usize>) {
    let mut run = 0;
    for ink in cells {
        if ink {
            run += 1;
        } else if run > 0 {
            out.push(run);
            run = 0;
        }
    }
    if run > 0 {
        out.push(run);
    }
}

impl Shape {
    /// Measures a picture of a line. `pixel(x, y)` gives 0xRRGGBB; a pixel is
    /// ink when it is closer to `ink` than to `background`. None when there is
    /// too little to measure (tiny text, a picture without letters).
    pub fn measure(width: usize, height: usize, pixel: impl Fn(usize, usize) -> u32, ink: u32, background: u32) -> Option<Shape> {
        if width == 0 || height == 0 || color_gap(ink, background) < 40 * 40 {
            return None; // letters indistinguishable from the background
        }
        let mask: Vec<bool> = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| {
                let c = pixel(x, y);
                color_gap(c, ink) < color_gap(c, background)
            })
            .collect();
        let at = |x: usize, y: usize| mask[y * width + x];
        let total = mask.iter().filter(|&&m| m).count();
        if total < MIN_INK {
            return None;
        }

        // Body of the letters: rows between the 10th and 90th percentile of the
        // ink, so accents, tails and stray pixels don't count.
        let rows: Vec<usize> = (0..height).map(|y| (0..width).filter(|&x| at(x, y)).count()).collect();
        let (mut seen, mut top, mut bottom) = (0, None, 0);
        for (y, n) in rows.iter().enumerate() {
            seen += n;
            if top.is_none() && seen * 10 >= total {
                top = Some(y);
            }
            if seen * 10 <= total * 9 {
                bottom = y;
            }
        }
        let top = top.unwrap_or(0);
        let body = bottom.saturating_sub(top) + 1;
        if body < MIN_BODY {
            return None;
        }

        // Ink box (for the density) and run lengths inside the body.
        let cols: Vec<usize> = (0..width).filter(|&x| (top..=bottom).any(|y| at(x, y))).collect();
        let (left, right) = (*cols.first()?, *cols.last()?);
        let mut across = Vec::new(); // horizontal runs: widths of vertical strokes
        for y in top..=bottom {
            runs((left..=right).map(|x| at(x, y)), &mut across);
        }
        let mut down = Vec::new(); // vertical runs: thickness of horizontal strokes
        for x in left..=right {
            runs((top..=bottom).map(|y| at(x, y)), &mut down);
        }
        // Vertical runs as tall as the body are stems, not horizontal strokes.
        down.retain(|&r| r * 10 < body * 6);
        let stem = median(across);
        let bar = median(down).max(1.0);
        let ink_in_body: usize = rows[top..=bottom].iter().sum();

        // Slant: the shear that lines the strokes up best (sharpest column profile).
        let center = (top + bottom) as f32 / 2.0;
        let slant = SHEARS
            .iter()
            .copied()
            .max_by_key(|&shear| {
                let mut columns = vec![0u32; width * 2];
                for y in top..=bottom {
                    let shift = shear * (y as f32 - center);
                    for x in left..=right {
                        if at(x, y) {
                            let col = (x as f32 + shift + width as f32 / 2.0).round();
                            if col >= 0.0 && (col as usize) < columns.len() {
                                columns[col as usize] += 1;
                            }
                        }
                    }
                }
                columns.iter().map(|&c| u64::from(c) * u64::from(c)).sum::<u64>()
            })
            .unwrap_or(0.0);

        Some(Shape {
            stroke: stem / body as f32,
            contrast: stem / bar,
            density: ink_in_body as f32 / ((right - left + 1) * body) as f32,
            slant,
        })
    }

    /// How different two shapes are: about 1 at the edge of "same font", more
    /// beyond. Each measure is scaled by how much it varies within one font
    /// (different words in the same font move these numbers a little).
    pub fn distance(&self, other: &Shape) -> f32 {
        let ratio = |a: f32, b: f32| (a.max(1e-3) / b.max(1e-3)).ln().abs();
        let parts = [
            ratio(self.stroke, other.stroke) / 0.30,
            ratio(self.contrast, other.contrast) / 0.40,
            (self.density - other.density).abs() / 0.12,
            (self.slant - other.slant).abs() / 0.12,
        ];
        parts.iter().sum::<f32>() / parts.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INK: u32 = 0xFFFFFF;
    const BG: u32 = 0x101010;

    /// A picture of a line of "letters": vertical stems `stem` px wide joined by
    /// horizontal bars `bar` px thick, `height` px tall, sheared by `lean`.
    fn line(stem: usize, bar: usize, height: usize, lean: f32, letters: usize) -> (usize, usize, Vec<bool>) {
        let pitch = height * 6 / 10 + stem * 2;
        let width = letters * pitch + height;
        let mut mask = vec![false; width * height];
        for y in 0..height {
            let shift = (lean * (height - 1 - y) as f32).round() as usize;
            for l in 0..letters {
                let x0 = l * pitch + shift;
                for x in x0..x0 + stem {
                    mask[y * width + x] = true; // left stem
                }
                for x in x0 + pitch / 2..x0 + pitch / 2 + stem {
                    mask[y * width + x] = true; // right stem
                }
                if y < bar || (height / 2..height / 2 + bar).contains(&y) {
                    for x in x0..x0 + pitch / 2 + stem {
                        mask[y * width + x] = true; // bars
                    }
                }
            }
        }
        (width, height, mask)
    }

    /// Measures a synthetic line.
    fn shape_of(stem: usize, bar: usize, height: usize, lean: f32, letters: usize) -> Shape {
        let (w, h, mask) = line(stem, bar, height, lean, letters);
        Shape::measure(w, h, |x, y| if mask[y * w + x] { INK } else { BG }, INK, BG).unwrap()
    }

    #[test]
    fn the_same_font_at_another_size_and_length_is_close() {
        let small = shape_of(3, 3, 30, 0.0, 4);
        let big = shape_of(6, 6, 60, 0.0, 9); // twice the size, other "word"
        assert!(small.distance(&big) < 1.0, "{small:?} vs {big:?}");
    }

    #[test]
    fn bold_and_regular_are_different() {
        let regular = shape_of(3, 3, 40, 0.0, 6);
        let bold = shape_of(8, 8, 40, 0.0, 6);
        assert!(regular.distance(&bold) > 1.0);
    }

    #[test]
    fn thin_bars_with_thick_stems_are_different() {
        let even = shape_of(5, 5, 40, 0.0, 6);
        let contrasted = shape_of(5, 1, 40, 0.0, 6); // serif-like contrast
        assert!(even.distance(&contrasted) > 1.0);
    }

    #[test]
    fn italic_is_seen() {
        let upright = shape_of(4, 4, 40, 0.0, 6);
        let italic = shape_of(4, 4, 40, 0.2, 6);
        assert!(italic.slant.abs() > 0.1, "{italic:?}");
        assert!(upright.slant.abs() < 0.06, "{upright:?}");
    }

    #[test]
    fn nothing_to_measure_gives_none() {
        assert!(Shape::measure(10, 10, |_, _| BG, INK, BG).is_none());
        assert!(Shape::measure(10, 10, |_, _| INK, INK, INK).is_none()); // no contrast
    }
}
