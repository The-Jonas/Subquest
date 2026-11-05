//! Text styles of a game, created by the user. Games write different things in
//! different fonts (menus, dialogue, names, song lyrics). The user takes a
//! print of a phrase in the game, picks the font that matches it, and from
//! then on every line written in that color uses that font, whatever its size
//! (menus grow the selected item, titles are bigger than the text below).
//!
//! Lines are matched mainly by the look of their letters (see shape.rs:
//! stroke weight, contrast, density, slant, all independent of size and
//! wording), and secondly by their color. Lines too small to measure fall back
//! to the color alone.
//!
//! Files: `<app data>/styles/<game>.json` and `<app data>/styles/<game>/<id>.png`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::shape::Shape;

pub const MAX_STYLES: usize = 24;
const SAME_COLOR: f64 = 60.0; // color-only matching: RGB distance under which a line belongs to a style
const SAME_SHAPE: f32 = 1.6; // shape distance beyond which a line is in another font
const COLOR_WEIGHT: f32 = 0.35; // how much a color difference of SAME_COLOR adds to the shape distance

/// One way the game writes text, defined by a print the user took.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TextStyle {
    pub id: String,
    pub name: String,   // given by the user ("" = unnamed)
    pub text_rgb: u32,  // letter color, measured on the print (0xRRGGBB)
    pub shape: Option<Shape>, // look of the letters, measured on the print
    pub seen_case: Option<String>, // letter case measured on the print ("", "upper", "smallCaps")
    pub font: String,   // chosen font ("" = not chosen yet)
    pub caps: String,   // letter case: "" = as in the game (seen_case), "none" = as translated, "upper", "smallCaps"
    pub manual: bool,   // taken by the user (styles from the old automatic version are dropped)
}

/// All the styles of one game.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct StyleBook {
    pub styles: Vec<TextStyle>,
    next_id: u32,
}

/// Distance between two 0xRRGGBB colors.
fn color_distance(a: u32, b: u32) -> f64 {
    let channel = |c: u32, shift: u32| ((c >> shift) & 0xFF) as f64;
    let d = |shift| channel(a, shift) - channel(b, shift);
    (d(16).powi(2) + d(8).powi(2) + d(0).powi(2)).sqrt()
}

impl StyleBook {
    /// The style a line belongs to. When both the line and a style have a
    /// measured shape, the letters decide (the color only nudges, so the same
    /// font in two colors is one style and two fonts of one color are two);
    /// otherwise the closest color within SAME_COLOR. Styles without a font yet
    /// are skipped (they would change nothing).
    pub fn style_for(&self, text_rgb: u32, shape: Option<&Shape>) -> Option<&TextStyle> {
        self.styles
            .iter()
            .filter(|s| !s.font.is_empty())
            .filter_map(|s| {
                let color = color_distance(s.text_rgb, text_rgb);
                match (shape, s.shape.as_ref()) {
                    (Some(line), Some(style)) => {
                        let d = line.distance(style);
                        (d <= SAME_SHAPE).then(|| (s, d + COLOR_WEIGHT * (color / SAME_COLOR) as f32))
                    }
                    _ => (color <= SAME_COLOR).then(|| (s, (color / SAME_COLOR) as f32)),
                }
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(s, _)| s)
    }

    /// Adds a style for a print the user took; returns its id. A full book
    /// drops its oldest style without a font first.
    pub fn add(&mut self, text_rgb: u32, shape: Option<Shape>, seen_case: &str) -> (String, Option<String>) {
        let evicted = if self.styles.len() >= MAX_STYLES {
            let i = self.styles.iter().position(|s| s.font.is_empty()).unwrap_or(0);
            Some(self.styles.remove(i).id)
        } else {
            None
        };
        self.next_id += 1;
        let id = format!("s{}", self.next_id);
        self.styles.push(TextStyle {
            id: id.clone(),
            text_rgb,
            shape,
            seen_case: Some(seen_case.to_string()),
            manual: true,
            ..Default::default()
        });
        (id, evicted)
    }
}

/// An RGBA picture (rows of width * 4 bytes).
pub struct Rgba {
    pub pixels: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

impl Rgba {
    /// Copies a rectangle (clipped to the picture); None when nothing is left.
    pub fn crop(&self, rect: [i32; 4]) -> Option<Rgba> {
        let x0 = rect[0].clamp(0, self.width);
        let y0 = rect[1].clamp(0, self.height);
        let x1 = (rect[0] + rect[2]).clamp(0, self.width);
        let y1 = (rect[1] + rect[3]).clamp(0, self.height);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let mut pixels = Vec::with_capacity(((x1 - x0) * (y1 - y0) * 4) as usize);
        for y in y0..y1 {
            let start = ((y * self.width + x0) * 4) as usize;
            pixels.extend_from_slice(&self.pixels[start..start + ((x1 - x0) * 4) as usize]);
        }
        Some(Rgba { pixels, width: x1 - x0, height: y1 - y0 })
    }

    /// PNG file bytes.
    pub fn png(&self) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, self.width as u32, self.height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&self.pixels).ok()?;
        }
        Some(out)
    }

    /// Color of a pixel as 0xRRGGBB.
    fn rgb(&self, x: i32, y: i32) -> u32 {
        let i = ((y * self.width + x) * 4) as usize;
        u32::from(self.pixels[i]) << 16 | u32::from(self.pixels[i + 1]) << 8 | u32::from(self.pixels[i + 2])
    }

    /// Reads a PNG file (a saved print).
    pub fn from_png(bytes: &[u8]) -> Option<Rgba> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().ok()?;
        let mut buf = vec![0; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut buf).ok()?;
        let (w, h) = (info.width as usize, info.height as usize);
        let channels = buf.len() / (w * h).max(1);
        if channels < 3 {
            return None;
        }
        let mut pixels = Vec::with_capacity(w * h * 4);
        for p in buf.chunks(channels).take(w * h) {
            pixels.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
        Some(Rgba { pixels, width: w as i32, height: h as i32 })
    }

    /// Look of the letters on the print (see shape.rs). A print may hold
    /// several lines (a whole menu): each line is measured on its own and the
    /// results are averaged, weighted by line height, since the measures are
    /// relative to one line's height.
    pub fn shape(&self) -> Option<Shape> {
        let (ink, background) = self.colors();
        let lines = self.line_bands(ink, background);
        let measured: Vec<(Shape, f32)> = lines
            .iter()
            .filter_map(|&(top, bottom)| {
                let pixel = |x: usize, y: usize| self.rgb(x as i32, (top + y) as i32);
                let shape = Shape::measure(self.width as usize, bottom - top, pixel, ink, background)?;
                Some((shape, (bottom - top) as f32))
            })
            .collect();
        let total: f32 = measured.iter().map(|(_, w)| w).sum();
        if total <= 0.0 {
            return None;
        }
        let avg = |f: fn(&Shape) -> f32| measured.iter().map(|(s, w)| f(s) * w).sum::<f32>() / total;
        Some(Shape {
            stroke: avg(|s| s.stroke),
            contrast: avg(|s| s.contrast),
            density: avg(|s| s.density),
            slant: avg(|s| s.slant),
        })
    }

    /// Letter case on the print (see shape::letter_case), voted line by line:
    /// the answer most lines give, ignoring lines that can't tell.
    pub fn letter_case(&self) -> &'static str {
        let (ink, background) = self.colors();
        let mut votes = std::collections::HashMap::new();
        for (top, bottom) in self.line_bands(ink, background) {
            let pixel = |x: usize, y: usize| self.rgb(x as i32, (top + y) as i32);
            let case = crate::shape::letter_case(self.width as usize, bottom - top, pixel, ink, background);
            *votes.entry(case).or_insert(0) += 1;
        }
        let decided = votes.iter().filter(|(case, _)| !case.is_empty()).max_by_key(|(_, n)| **n);
        match decided {
            Some((case, n)) if *n * 2 >= votes.values().sum::<i32>() => case,
            _ => "",
        }
    }

    /// Rows of each line of text on the print, as [top, bottom): runs of rows
    /// with ink, split where rows are empty. Small gaps (an accent above its
    /// letter) don't split a line.
    fn line_bands(&self, ink: u32, background: u32) -> Vec<(usize, usize)> {
        let is_ink = |c: u32| color_distance(c, ink) < color_distance(c, background);
        let rows: Vec<bool> = (0..self.height)
            .map(|y| (0..self.width).filter(|&x| is_ink(self.rgb(x, y))).count() > 1)
            .collect();
        let mut bands: Vec<(usize, usize)> = Vec::new();
        let mut start = None;
        for (y, &has_ink) in rows.iter().chain(std::iter::once(&false)).enumerate() {
            match (has_ink, start) {
                (true, None) => start = Some(y),
                (false, Some(top)) => {
                    bands.push((top, y));
                    start = None;
                }
                _ => {}
            }
        }
        // Joins pieces closer than a quarter of the tallest piece (accents, dots).
        let tallest = bands.iter().map(|(t, b)| b - t).max().unwrap_or(0);
        let mut lines: Vec<(usize, usize)> = Vec::new();
        for band in bands {
            match lines.last_mut() {
                Some(last) if (band.0 - last.1) * 4 < tallest => last.1 = band.1,
                _ => lines.push(band),
            }
        }
        lines
    }

    /// Color of the letters on the print.
    pub fn ink_color(&self) -> u32 {
        self.colors().0
    }

    /// Letter and background colors: the background is the median of the
    /// border, and the letters are the pixels farthest from it (the top 12%), averaged.
    fn colors(&self) -> (u32, u32) {
        let px = |x: i32, y: i32| self.rgb(x, y);
        let mut border = Vec::new();
        for x in 0..self.width {
            border.push(px(x, 0));
            border.push(px(x, self.height - 1));
        }
        for y in 0..self.height {
            border.push(px(0, y));
            border.push(px(self.width - 1, y));
        }
        let median = |shift: u32| {
            let mut v: Vec<u32> = border.iter().map(|c| (c >> shift) & 0xFF).collect();
            v.sort_unstable();
            v[v.len() / 2]
        };
        let background = median(16) << 16 | median(8) << 8 | median(0);

        let mut all: Vec<(f64, u32)> = (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .map(|(x, y)| {
                let c = px(x, y);
                (color_distance(c, background), c)
            })
            .collect();
        all.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        let top = &all[..(all.len() * 12 / 100).max(1)];
        let avg = |shift: u32| top.iter().map(|(_, c)| (c >> shift) & 0xFF).sum::<u32>() / top.len() as u32;
        (avg(16) << 16 | avg(8) << 8 | avg(0), background)
    }
}

/// The style books on disk, with the game being translated kept in memory.
pub struct StyleStore {
    dir: PathBuf,
    open: Mutex<Option<(String, StyleBook)>>, // (game key, its book)
    // Last lines read in each area (event id, color and shape per line), so a
    // style change can restyle what is on screen now: a static menu is never
    // read again, and would otherwise keep its old font until it changes.
    last: Mutex<std::collections::HashMap<u32, (u64, Vec<(u32, Option<Shape>)>)>>,
}

/// File-safe version of a game key ("hades2.exe" stays, odd characters become "_").
fn file_key(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect()
}

impl StyleStore {
    /// Store rooted at `<app data>/styles`.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            dir: data_dir.join("styles"),
            open: Mutex::new(None),
            last: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn book_path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{}.json", file_key(key)))
    }

    /// Path of a style's print.
    pub fn print_path(&self, key: &str, id: &str) -> PathBuf {
        self.dir.join(file_key(key)).join(format!("{}.png", file_key(id)))
    }

    /// Reads a game's book, leaving out styles from the old automatic version.
    fn load(&self, key: &str) -> StyleBook {
        let mut book: StyleBook = std::fs::read_to_string(self.book_path(key))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        book.styles.retain(|s| s.manual);
        book
    }

    fn save(&self, key: &str, book: &StyleBook) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.dir).map_err(|e| AppError::other(e.to_string()))?;
        let text = serde_json::to_string_pretty(book).map_err(|e| AppError::other(e.to_string()))?;
        std::fs::write(self.book_path(key), text).map_err(|e| AppError::other(e.to_string()))
    }

    /// Runs `change` on a game's book (the open one, or loaded from disk) and saves it.
    pub fn update<T>(&self, key: &str, change: impl FnOnce(&mut StyleBook) -> T) -> Result<T, AppError> {
        let mut open = self.open.lock().unwrap();
        if let Some((k, book)) = open.as_mut().filter(|(k, _)| k == key) {
            let result = change(book);
            self.save(k, book)?;
            return Ok(result);
        }
        let mut book = self.load(key);
        let result = change(&mut book);
        self.save(key, &book)?;
        Ok(result)
    }

    /// A game's styles, in the order they were taken.
    pub fn list(&self, key: &str) -> Vec<TextStyle> {
        let open = self.open.lock().unwrap();
        match open.as_ref() {
            Some((k, book)) if k == key => book.styles.clone(),
            _ => self.load(key).styles,
        }
    }

    /// Keeps a game's book in memory while it is translated. Styles taken
    /// before shapes were measured get theirs from their print now.
    pub fn open(&self, key: &str) {
        let mut book = self.load(key);
        let mut measured = false;
        for style in book.styles.iter_mut().filter(|s| s.shape.is_none() || s.seen_case.is_none()) {
            let print = std::fs::read(self.print_path(key, &style.id)).ok();
            let Some(print) = print.as_deref().and_then(Rgba::from_png) else { continue };
            style.shape = style.shape.or_else(|| print.shape());
            style.seen_case = Some(print.letter_case().to_string());
            measured = true;
        }
        if measured {
            let _ = self.save(key, &book);
        }
        *self.open.lock().unwrap() = Some((key.to_string(), book));
    }

    /// Adds a style from a print (already cropped); saves the print and the book.
    pub fn add_from_print(&self, key: &str, print: &Rgba) -> Result<String, AppError> {
        let png = print.png().ok_or_else(|| AppError::other("could not encode the print"))?;
        let color = print.ink_color();
        let shape = print.shape();
        let case = print.letter_case();
        let (id, evicted) = self.update(key, |book| book.add(color, shape, case))?;
        let path = self.print_path(key, &id);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| AppError::other(e.to_string()))?;
        }
        std::fs::write(&path, png).map_err(|e| AppError::other(e.to_string()))?;
        if let Some(old) = evicted {
            let _ = std::fs::remove_file(self.print_path(key, &old));
        }
        Ok(id)
    }

    /// Style id of each line (letter color and, when measurable, letter shape),
    /// for the game being translated. Remembers them as the area's lines on screen.
    pub fn match_lines(&self, area: u32, event: u64, lines: Vec<(u32, Option<Shape>)>) -> Vec<Option<String>> {
        let ids = self.ids_for(&lines);
        self.last.lock().unwrap().insert(area, (event, lines));
        ids
    }

    /// Style ids for lines, with the open book (all None without one).
    fn ids_for(&self, lines: &[(u32, Option<Shape>)]) -> Vec<Option<String>> {
        let open = self.open.lock().unwrap();
        let Some((_, book)) = open.as_ref() else {
            return vec![None; lines.len()];
        };
        lines
            .iter()
            .map(|(color, shape)| book.style_for(*color, shape.as_ref()).map(|s| s.id.clone()))
            .collect()
    }

    /// The lines on screen matched again after a style changed: (event id,
    /// style id per line) for each area.
    pub fn rematch(&self) -> Vec<(u64, Vec<Option<String>>)> {
        let last = self.last.lock().unwrap().clone();
        last.into_values().map(|(event, lines)| (event, self.ids_for(&lines))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREEN: u32 = 0x3DE8A0;

    /// A book with one green style that has a font.
    fn book() -> StyleBook {
        let mut book = StyleBook::default();
        let (id, _) = book.add(GREEN, None, "");
        book.styles.iter_mut().find(|s| s.id == id).unwrap().font = "imported:Cinzel-700.ttf".into();
        book
    }

    #[test]
    fn lines_of_close_colors_use_the_style_whatever_their_size() {
        let book = book();
        assert!(book.style_for(0x40E0A8, None).is_some()); // anti-aliasing / glow shifts the color a bit
        assert!(book.style_for(0xE9E4D6, None).is_none()); // cream dialogue is another style
    }

    #[test]
    fn a_style_without_a_font_changes_nothing() {
        let mut book = StyleBook::default();
        book.add(GREEN, None, "");
        assert!(book.style_for(GREEN, None).is_none());
    }

    /// A style with a font and a measured shape.
    fn styled(book: &mut StyleBook, color: u32, shape: Shape, font: &str) -> String {
        let (id, _) = book.add(color, Some(shape), "");
        book.styles.iter_mut().find(|s| s.id == id).unwrap().font = font.into();
        id
    }

    #[test]
    fn the_letters_decide_before_the_color() {
        let regular = Shape { stroke: 0.17, contrast: 1.0, density: 0.30, slant: 0.0 };
        let bold = Shape { stroke: 0.40, contrast: 1.6, density: 0.50, slant: 0.0 };
        let mut book = StyleBook::default();
        let body = styled(&mut book, 0xE9E4D6, regular, "body");
        let title = styled(&mut book, 0xE9E4D6, bold, "title");
        // Same font as the body, in green (a highlighted line): still the body style.
        assert_eq!(book.style_for(0x3DE8A0, Some(&regular)).map(|s| s.id.clone()), Some(body));
        // Same color as both: the bold letters pick the title style.
        assert_eq!(book.style_for(0xE9E4D6, Some(&bold)).map(|s| s.id.clone()), Some(title));
        // With an italic style taken too, slanted lines go to it.
        let italic = Shape { slant: 0.2, ..regular };
        let quote = styled(&mut book, 0xE9E4D6, italic, "quote");
        assert_eq!(book.style_for(0xE9E4D6, Some(&italic)).map(|s| s.id.clone()), Some(quote));
        // Something in no way like any style (huge strokes, no slant) gets none.
        let poster = Shape { stroke: 0.9, contrast: 4.0, density: 0.9, slant: -0.3 };
        assert!(book.style_for(0xE9E4D6, Some(&poster)).is_none());
    }

    /// A print with `lines` lines of "letters" (stems 3 px wide, bars 3 px
    /// thick, `body` px tall), `gap` px apart.
    fn menu_print(lines: usize, body: i32, gap: i32) -> Rgba {
        let (width, height) = (120, 10 + lines as i32 * (body + gap));
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let line_y = (y - 10).rem_euclid(body + gap);
                let in_line = y >= 10 && line_y < body;
                let stem = (x % 14) < 3;
                let bar = line_y < 3 || (body / 2..body / 2 + 3).contains(&line_y);
                let ink = in_line && (10..110).contains(&x) && (stem || bar);
                pixels.extend_from_slice(if ink { &[0xEB, 0xE4, 0xD6, 255] } else { &[16, 20, 24, 255] });
            }
        }
        Rgba { pixels, width, height }
    }

    #[test]
    fn a_print_of_a_whole_menu_measures_like_one_line() {
        let one = menu_print(1, 30, 18).shape().unwrap();
        let three = menu_print(3, 30, 18).shape().unwrap();
        assert!(one.distance(&three) < 0.3, "{one:?} vs {three:?}");
    }

    #[test]
    fn tells_small_capitals_from_ordinary_text_on_a_print() {
        // Letters: (x, width, rise above the baseline, hang below it).
        let draw = |letters: &[(i32, i32, i32, i32)]| {
            let (width, height, baseline) = (200, 70, 50);
            let mut pixels = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    let ink = letters.iter().any(|&(lx, w, rise, hang)| {
                        (lx..lx + w).contains(&x) && y > baseline - rise && y <= baseline + hang
                    });
                    pixels.extend_from_slice(if ink { &[0xEB, 0xE4, 0xD6, 255] } else { &[16, 20, 24, 255] });
                }
            }
            Rgba { pixels, width, height }
        };
        // "PLAY" in small capitals: one tall letter, then short ones, nothing hangs.
        let small_caps = draw(&[(10, 14, 30, 0), (30, 14, 22, 0), (50, 14, 22, 0), (70, 14, 22, 0)]);
        assert_eq!(small_caps.letter_case(), "smallCaps");
        // "Play": the l is tall inside the word and the y hangs.
        let ordinary = draw(&[(10, 14, 30, 0), (30, 6, 32, 0), (42, 14, 22, 0), (62, 14, 22, 8)]);
        assert_eq!(ordinary.letter_case(), "");
        // "PLAY": all capitals.
        let upper = draw(&[(10, 14, 30, 0), (30, 14, 30, 0), (50, 14, 30, 0), (70, 14, 30, 0)]);
        assert_eq!(upper.letter_case(), "upper");
    }

    #[test]
    fn measures_the_letter_color_on_a_print() {
        // Dark 40x20 picture with a bright green bar in the middle (the "letters").
        let mut pixels = Vec::new();
        for y in 0..20 {
            for x in 0..40 {
                let ink = (8..32).contains(&x) && (8..12).contains(&y);
                pixels.extend_from_slice(if ink { &[0x3D, 0xE8, 0xA0, 255] } else { &[12, 20, 16, 255] });
            }
        }
        let print = Rgba { pixels, width: 40, height: 20 };
        assert!(color_distance(print.ink_color(), GREEN) < 20.0);
        let png = print.png().unwrap();
        assert_eq!(&png[1..4], b"PNG");
        let back = Rgba::from_png(&png).unwrap();
        assert_eq!((back.width, back.height, back.pixels.len()), (40, 20, 40 * 20 * 4));
        assert!(print.crop([100, 100, 5, 5]).is_none());
        assert_eq!(print.crop([30, 15, 20, 20]).map(|c| (c.width, c.height)), Some((10, 5)));
    }
}
