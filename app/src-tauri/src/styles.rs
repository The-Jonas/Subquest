//! Text styles of a game, created by the user. Games write different things in
//! different fonts (menus, dialogue, names, song lyrics). The user takes a
//! print of a phrase in the game, picks the font that matches it, and from
//! then on every line written in that color uses that font, whatever its size
//! (menus grow the selected item, titles are bigger than the text below).
//!
//! Lines are matched by their letter color only: it is what tells a game's
//! styles apart, and the size is not (an automatic version that also used the
//! size kept splitting one font into many styles).
//!
//! Files: `<app data>/styles/<game>.json` and `<app data>/styles/<game>/<id>.png`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const MAX_STYLES: usize = 24;
const SAME_COLOR: f64 = 60.0; // RGB distance under which a line belongs to a style

/// One way the game writes text, defined by a print the user took.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TextStyle {
    pub id: String,
    pub name: String,   // given by the user ("" = unnamed)
    pub text_rgb: u32,  // letter color, measured on the print (0xRRGGBB)
    pub font: String,   // chosen font ("" = not chosen yet)
    pub caps: String,   // letter case chosen with the font
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
    /// The style a line in this color belongs to: the closest color, if close enough.
    /// Styles without a font yet are skipped (they would change nothing).
    pub fn style_for(&self, text_rgb: u32) -> Option<&TextStyle> {
        self.styles
            .iter()
            .filter(|s| !s.font.is_empty())
            .map(|s| (s, color_distance(s.text_rgb, text_rgb)))
            .filter(|(_, d)| *d <= SAME_COLOR)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(s, _)| s)
    }

    /// Adds a style for a print the user took; returns its id. A full book
    /// drops its oldest style without a font first.
    pub fn add(&mut self, text_rgb: u32) -> (String, Option<String>) {
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

    /// Color of the letters: the background is the median of the border, and
    /// the letters are the pixels farthest from it (the top 12%), averaged.
    pub fn ink_color(&self) -> u32 {
        let px = |x: i32, y: i32| {
            let i = ((y * self.width + x) * 4) as usize;
            u32::from(self.pixels[i]) << 16 | u32::from(self.pixels[i + 1]) << 8 | u32::from(self.pixels[i + 2])
        };
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
        avg(16) << 16 | avg(8) << 8 | avg(0)
    }
}

/// The style books on disk, with the game being translated kept in memory.
pub struct StyleStore {
    dir: PathBuf,
    open: Mutex<Option<(String, StyleBook)>>, // (game key, its book)
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

    /// Keeps a game's book in memory while it is translated.
    pub fn open(&self, key: &str) {
        *self.open.lock().unwrap() = Some((key.to_string(), self.load(key)));
    }

    /// Adds a style from a print (already cropped); saves the print and the book.
    pub fn add_from_print(&self, key: &str, print: &Rgba) -> Result<String, AppError> {
        let png = print.png().ok_or_else(|| AppError::other("could not encode the print"))?;
        let color = print.ink_color();
        let (id, evicted) = self.update(key, |book| book.add(color))?;
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

    /// Style id of each line color, for the game being translated.
    pub fn match_lines(&self, colors: &[u32]) -> Vec<Option<String>> {
        let open = self.open.lock().unwrap();
        let Some((_, book)) = open.as_ref() else {
            return vec![None; colors.len()];
        };
        colors.iter().map(|c| book.style_for(*c).map(|s| s.id.clone())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREEN: u32 = 0x3DE8A0;

    /// A book with one green style that has a font.
    fn book() -> StyleBook {
        let mut book = StyleBook::default();
        let (id, _) = book.add(GREEN);
        book.styles.iter_mut().find(|s| s.id == id).unwrap().font = "imported:Cinzel-700.ttf".into();
        book
    }

    #[test]
    fn lines_of_close_colors_use_the_style_whatever_their_size() {
        let book = book();
        assert!(book.style_for(0x40E0A8).is_some()); // anti-aliasing / glow shifts the color a bit
        assert!(book.style_for(0xE9E4D6).is_none()); // cream dialogue is another style
    }

    #[test]
    fn a_style_without_a_font_changes_nothing() {
        let mut book = StyleBook::default();
        book.add(GREEN);
        assert!(book.style_for(GREEN).is_none());
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
        assert_eq!(&print.png().unwrap()[1..4], b"PNG");
        assert!(print.crop([100, 100, 5, 5]).is_none());
        assert_eq!(print.crop([30, 15, 20, 20]).map(|c| (c.width, c.height)), Some((10, 5)));
    }
}
