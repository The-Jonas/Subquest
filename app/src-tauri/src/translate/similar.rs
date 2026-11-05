//! Tells when two OCR readings are the same line. On animated backgrounds OCR
//! reads the same text a little differently from frame to frame ("parar" /
//! "parar." / "pararl"); those must reuse one translation instead of starting
//! a new request and blanking the overlay while it runs.

/// Shortest text that is worth translating: at least this many letters.
const MIN_LETTERS: usize = 2;
/// Two readings at least this similar (0..1) are the same line.
const SAME_LINE: f64 = 0.85;

/// Lowercase letters and digits only, words separated by one space.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for word in text.split_whitespace() {
        let clean: String = word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
        if clean.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&clean);
    }
    out
}

/// True when the text has real words, not a few stray marks from the scenery.
pub fn meaningful(text: &str) -> bool {
    text.chars().filter(|c| c.is_alphabetic()).count() >= MIN_LETTERS
}

/// True when two normalized readings are the same line (small OCR differences only).
pub fn same_line(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (la, lb) = (a.chars().count(), b.chars().count());
    let longest = la.max(lb);
    if longest == 0 {
        return false;
    }
    // Lengths too far apart can't reach the threshold: skip the full comparison.
    if (la.abs_diff(lb) as f64) > longest as f64 * (1.0 - SAME_LINE) {
        return false;
    }
    1.0 - levenshtein(a, b) as f64 / longest as f64 >= SAME_LINE
}

/// Edit distance between two strings, in characters (two-row dynamic programming).
fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut row = vec![0; b.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != *cb);
            row[j + 1] = substitution.min(previous[j + 1] + 1).min(row[j] + 1);
        }
        std::mem::swap(&mut previous, &mut row);
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_drops_case_and_punctuation() {
        assert_eq!(normalize("  That spin, ALWAYS... spin!  "), "that spin always spin");
    }

    #[test]
    fn ocr_noise_is_the_same_line() {
        let a = normalize("That spin and spin forevermore");
        let b = normalize("That spin and spln forevermore.");
        assert!(same_line(&a, &b));
    }

    #[test]
    fn a_growing_line_is_a_new_line() {
        // Typewriter effect: the full line must be translated again.
        assert!(!same_line(&normalize("That spin"), &normalize("That spin and spin forevermore")));
    }

    #[test]
    fn stray_marks_are_not_text() {
        assert!(!meaningful("| ."));
        assert!(!meaningful("a ."));
        assert!(meaningful("Oh!"));
    }
}
