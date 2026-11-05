//! Text styles of a game. Games write different things in different fonts
//! (menus, dialogue, names, song lyrics), and the place on screen doesn't
//! always tell them apart, especially when the whole screen is read. What does
//! tell them apart, cheaply, is how each line looks: its text color and its
//! height. Lines with close colors and heights are one "style".
//!
//! The first time a style shows up, a picture of that line is kept (the
//! "print"), so the user can compare fonts with the real thing and pick one for
//! that style only. Each game keeps at most MAX_STYLES; when a new one comes
//! and the book is full, the least seen style without a chosen font goes.
//!
//! Files: `<app data>/styles/<game>.json` and `<app data>/styles/<game>/<id>.png`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const MAX_STYLES: usize = 24;
const SAME_COLOR: f64 = 48.0; // RGB distance under which two text colors are the same style
// Height ratio under which two lines are the same style. Very generous on
// purpose: OCR boxes hug the letters ("PLAY", with a tail, is taller than
// "QUIT") and menus grow the selected item, so one font shows up in several
// sizes. Color tells styles apart much better than size.
const SAME_HEIGHT: f64 = 1.8;
const PRINT_MARGIN: f64 = 0.35; // context kept around the line in its print, in line heights

/// One way the game writes text.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TextStyle {
    pub id: String,
    pub text_rgb: u32,  // letter color (0xRRGGBB)
    pub height: i32,    // line height in window pixels (the first time it was seen)
    pub sample: String, // the line that was printed, to draw font samples with
    pub area: u32,      // area it was first seen in
    pub seen: u32,      // lines seen in this style
    pub last_seen: u64, // ms since epoch
    pub font: String,   // font chosen by the user ("" = not chosen)
    pub caps: String,   // letter case chosen with the font
    pub auto_font: String, // closest installed font, found by the app ("" = not yet)
    pub new: bool,      // not yet seen in the Styles screen
}

/// All the styles of one game.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct StyleBook {
    pub styles: Vec<TextStyle>,
    next_id: u32,
}

/// What happened when a line was matched against the book.
pub struct Observed {
    pub id: String,
    pub is_new: bool,              // a new style: its print must be saved
    pub evicted: Option<String>,   // style dropped to make room (delete its print)
}

/// Distance between two 0xRRGGBB colors.
fn color_distance(a: u32, b: u32) -> f64 {
    let channel = |c: u32, shift: u32| ((c >> shift) & 0xFF) as f64;
    let d = |shift| channel(a, shift) - channel(b, shift);
    (d(16).powi(2) + d(8).powi(2) + d(0).powi(2)).sqrt()
}

impl StyleBook {
    /// Index of the style a line belongs to, if any (closest color among the matches).
    pub fn find(&self, text_rgb: u32, height: i32) -> Option<usize> {
        let h = height.max(1) as f64;
        self.styles
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                let ratio = h.max(s.height as f64) / h.min(s.height.max(1) as f64);
                ratio <= SAME_HEIGHT && color_distance(s.text_rgb, text_rgb) <= SAME_COLOR
            })
            .min_by(|(_, a), (_, b)| {
                color_distance(a.text_rgb, text_rgb).total_cmp(&color_distance(b.text_rgb, text_rgb))
            })
            .map(|(i, _)| i)
    }

    /// Counts a line in its style, or starts a new style for it.
    pub fn observe(&mut self, text_rgb: u32, height: i32, sample: &str, area: u32, now: u64) -> Observed {
        if let Some(i) = self.find(text_rgb, height) {
            let style = &mut self.styles[i];
            style.seen += 1;
            style.last_seen = now;
            return Observed {
                id: style.id.clone(),
                is_new: false,
                evicted: None,
            };
        }
        let evicted = if self.styles.len() >= MAX_STYLES { self.evict() } else { None };
        self.next_id += 1;
        let id = format!("s{}", self.next_id);
        self.styles.push(TextStyle {
            id: id.clone(),
            text_rgb,
            height,
            sample: sample.chars().take(80).collect(),
            area,
            seen: 1,
            last_seen: now,
            new: true,
            ..Default::default()
        });
        Observed {
            id,
            is_new: true,
            evicted,
        }
    }

    /// Drops the least seen style (oldest first among ties), keeping the ones
    /// the user chose a font for; returns its id.
    fn evict(&mut self) -> Option<String> {
        let (i, _) = self
            .styles
            .iter()
            .enumerate()
            .filter(|(_, s)| s.font.is_empty())
            .min_by_key(|(_, s)| (s.seen, s.last_seen))?;
        Some(self.styles.remove(i).id)
    }
}

/// A borrowed picture of an area, as the core sends it (BGRA rows).
pub struct AreaImage<'a> {
    pub bgra: &'a [u8],
    pub x: i32, // top-left in window pixels
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
}

/// PNG of a line (window pixels) with a little context around it.
pub fn print_png(image: &AreaImage, rect: [i32; 4]) -> Option<Vec<u8>> {
    let margin = (rect[3] as f64 * PRINT_MARGIN).round() as i32;
    let x0 = (rect[0] - image.x - margin).clamp(0, image.width);
    let y0 = (rect[1] - image.y - margin).clamp(0, image.height);
    let x1 = (rect[0] - image.x + rect[2] + margin).clamp(0, image.width);
    let y1 = (rect[1] - image.y + rect[3] + margin).clamp(0, image.height);
    let (w, h) = (x1 - x0, y1 - y0);
    if w <= 0 || h <= 0 {
        return None;
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in y0..y1 {
        let row = (y as usize) * image.stride as usize;
        for x in x0..x1 {
            let p = row + (x as usize) * 4;
            let px = image.bgra.get(p..p + 4)?;
            rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w as u32, h as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&rgba).ok()?;
    }
    Some(out)
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

    fn load(&self, key: &str) -> StyleBook {
        std::fs::read_to_string(self.book_path(key))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self, key: &str, book: &StyleBook) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.dir).map_err(|e| AppError::other(e.to_string()))?;
        let text = serde_json::to_string_pretty(book).map_err(|e| AppError::other(e.to_string()))?;
        std::fs::write(self.book_path(key), text).map_err(|e| AppError::other(e.to_string()))
    }

    /// Runs `change` on a game's book (loading it if needed) and saves it.
    /// The book of the game being translated stays in memory.
    pub fn update<T>(&self, key: &str, change: impl FnOnce(&mut StyleBook) -> T) -> Result<T, AppError> {
        let mut open = self.open.lock().unwrap();
        if open.as_ref().map(|(k, _)| k.as_str()) != Some(key) {
            let mut book = self.load(key);
            let result = change(&mut book);
            self.save(key, &book)?;
            return Ok(result);
        }
        let (_, book) = open.as_mut().unwrap();
        let result = change(book);
        self.save(key, book)?;
        Ok(result)
    }

    /// A game's styles (from memory when it's the one being translated).
    pub fn list(&self, key: &str) -> Vec<TextStyle> {
        let open = self.open.lock().unwrap();
        match open.as_ref() {
            Some((k, book)) if k == key => book.styles.clone(),
            _ => self.load(key).styles,
        }
    }

    /// Keeps a game's book in memory for a session (saving the previous one).
    pub fn open(&self, key: &str) {
        let mut open = self.open.lock().unwrap();
        if let Some((k, book)) = open.take() {
            let _ = self.save(&k, &book);
        }
        *open = Some((key.to_string(), self.load(key)));
    }

    /// Saves the open book (the "seen" counters change all the time, so they
    /// are only written at the end of a session and when a style is added).
    pub fn flush(&self) {
        if let Some((k, book)) = self.open.lock().unwrap().as_ref() {
            let _ = self.save(k, book);
        }
    }

    /// Matches each line of an OCR result against the open book. New styles get
    /// their print saved. Returns each line's style id and whether anything new
    /// appeared (the UI then reloads the styles).
    pub fn observe_lines(
        &self,
        lines: &[(u32, [i32; 4], String)], // (text color, rect, text) per line
        area: u32,
        image: Option<&AreaImage>,
        now: u64,
    ) -> (Vec<Option<String>>, bool) {
        let mut open = self.open.lock().unwrap();
        let Some((key, book)) = open.as_mut() else {
            return (vec![None; lines.len()], false);
        };
        let mut changed = false;
        let ids = lines
            .iter()
            .map(|(rgb, rect, text)| {
                let seen = book.observe(*rgb, rect[3], text, area, now);
                if seen.is_new {
                    changed = true;
                    if let Some(png) = image.and_then(|img| print_png(img, *rect)) {
                        let path = self.print_path(key, &seen.id);
                        if let Some(dir) = path.parent() {
                            let _ = std::fs::create_dir_all(dir);
                        }
                        let _ = std::fs::write(path, png);
                    }
                }
                if let Some(old) = seen.evicted {
                    let _ = std::fs::remove_file(self.print_path(key, &old));
                }
                Some(seen.id)
            })
            .collect();
        if changed {
            let _ = self.save(key, book);
        }
        (ids, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CREAM: u32 = 0xE9E4D6;
    const GOLD: u32 = 0xF2C14E;

    #[test]
    fn close_color_and_size_is_the_same_style() {
        let mut book = StyleBook::default();
        let a = book.observe(CREAM, 40, "Hello", 0, 1);
        let b = book.observe(0xE2DDD0, 44, "There", 0, 2); // OCR noise in color and size
        assert!(a.is_new && !b.is_new);
        assert_eq!(a.id, b.id);
        assert_eq!(book.styles[0].seen, 2);
    }

    #[test]
    fn another_color_or_size_is_a_new_style() {
        let mut book = StyleBook::default();
        book.observe(CREAM, 40, "Dialogue", 0, 1);
        assert!(book.observe(GOLD, 40, "Name", 0, 2).is_new); // same size, other color
        assert!(book.observe(CREAM, 80, "TITLE", 0, 3).is_new); // same color, twice as big
        assert_eq!(book.styles.len(), 3);
    }

    #[test]
    fn a_menu_item_that_grows_when_selected_is_the_same_style() {
        let mut book = StyleBook::default();
        let normal = book.observe(0x3DE8A0, 43, "PLAY", 0, 1);
        let selected = book.observe(0x3DE8A0, 72, "OPTIONS", 0, 2); // Hades zooms the selected item
        assert_eq!(normal.id, selected.id);
    }

    #[test]
    fn a_full_book_drops_the_least_seen_unchosen_style() {
        let mut book = StyleBook::default();
        for i in 0..MAX_STYLES {
            // Colors 60 apart on a grid, so every one is a new style.
            let (r, g, b) = ((i % 5) as u32 * 60, (i / 5 % 5) as u32 * 60, (i / 25) as u32 * 60);
            book.observe(r << 16 | g << 8 | b, 40, "x", 0, i as u64);
        }
        book.styles[0].font = "Cinzel".into(); // chosen: must survive
        for s in book.styles.iter_mut().skip(1) {
            s.seen = 5;
        }
        book.styles[1].seen = 1; // least seen without a font
        let dropped = book.styles[1].id.clone();
        let new = book.observe(0x1E1EFF, 40, "new", 0, 99);
        assert!(new.is_new);
        assert_eq!(new.evicted, Some(dropped));
        assert_eq!(book.styles.len(), MAX_STYLES);
        assert_eq!(book.styles[0].font, "Cinzel");
        assert_eq!(book.styles.len(), MAX_STYLES);
    }

    #[test]
    fn prints_the_line_with_some_context() {
        // 20x10 area at window (100, 50), all one color.
        let bgra = vec![10u8; 20 * 10 * 4];
        let image = AreaImage { bgra: &bgra, x: 100, y: 50, width: 20, height: 10, stride: 80 };
        let png = print_png(&image, [104, 52, 8, 4]).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        assert!(print_png(&image, [500, 500, 8, 4]).is_none()); // outside the area
    }
}
