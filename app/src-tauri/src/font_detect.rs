//! Finds the font a game uses: the translator model looks at a crop of the
//! game's text and names the closest Google Fonts families; each one is then
//! downloaded (free, open licenses) so the user can compare them with the
//! game's text and keep the best one in the Fonts folder.
//!
//! Downloaded candidates stay in memory until the user picks one, so trying
//! the detection several times doesn't fill the Fonts folder.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use futures_util::future::join_all;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::translate::openai_compat::OpenAiCompatTranslator;
use crate::translate::{language_name, TranslateError};

const ASKED: usize = 5; // families asked from the model (some may not exist)
const SHOWN: usize = 3; // candidates shown to the user
const CSS_API: &str = "https://fonts.googleapis.com/css2?family=";
const TIMEOUT: Duration = Duration::from_secs(15);

/// One family named by the model.
#[derive(Debug, Deserialize, PartialEq)]
struct Guess {
    family: String,
    #[serde(default = "regular")]
    weight: u16,
    #[serde(default)]
    reason: String,
}

/// Weight used when the model doesn't say one.
fn regular() -> u16 {
    400
}

/// The model's whole answer.
#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    text: String, // a line of the game text it read, to draw the samples with
    #[serde(default)]
    case: String, // "normal" | "upper" | "smallCaps"
    fonts: Vec<Guess>,
}

/// A downloaded candidate, as the UI shows it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontCandidate {
    pub family: String,
    pub weight: u16,
    pub reason: String, // why it matches, in the UI language
    pub file: String,   // file name it gets when the user keeps it (also its id)
}

/// Result of a detection: the text the model read and the fonts it suggests.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontDetection {
    pub text: String,
    pub caps: String, // letter case the game uses: "" | "upper" | "smallCaps" (as in GameArea)
    pub candidates: Vec<FontCandidate>,
}

/// Candidates downloaded by the last detection, waiting for the user's choice.
#[derive(Default)]
pub struct FontFinder {
    downloaded: Mutex<HashMap<String, Vec<u8>>>, // file name -> font bytes
}

impl FontFinder {
    /// Bytes of a downloaded candidate (to preview it, or to keep it).
    pub fn bytes(&self, file: &str) -> Option<Vec<u8>> {
        self.downloaded.lock().unwrap().get(file).cloned()
    }

    /// Asks the model about the image, downloads what it names and keeps the
    /// first few that exist on Google Fonts, best match first.
    pub async fn detect(
        &self,
        model: &OpenAiCompatTranslator,
        http: &reqwest::Client,
        png: &[u8],
        ui_language: &str,
    ) -> Result<FontDetection, AppError> {
        let answer = model
            .ask_about_image(&prompt(ui_language), png)
            .await
            .map_err(AppError::from_translate)?;
        let (text, caps, guesses) = parse_answer(&answer).map_err(AppError::from_translate)?;

        let downloads = join_all(guesses.iter().map(|g| download(http, &g.family, g.weight))).await;
        let mut found = Vec::new();
        let mut downloaded = HashMap::new();
        for (guess, result) in guesses.into_iter().zip(downloads) {
            let Some((file, weight, bytes)) = result else { continue };
            if downloaded.contains_key(&file) || found.len() == SHOWN {
                continue;
            }
            downloaded.insert(file.clone(), bytes);
            found.push(FontCandidate {
                family: guess.family,
                weight,
                reason: guess.reason,
                file,
            });
        }
        if found.is_empty() {
            return Err(AppError {
                kind: "fontNotFound",
                detail: format!("none of the suggested fonts could be downloaded: {answer}"),
            });
        }
        *self.downloaded.lock().unwrap() = downloaded;
        Ok(FontDetection {
            text,
            caps,
            candidates: found,
        })
    }
}

/// What the model is asked. It is told to judge only the letter shapes: left
/// alone, models pick "themed" fonts (a Greek-myth game gets Greek-looking
/// decorative fonts even when its menus use a plain sans-serif). Classifying
/// the letters first makes the names that follow much closer.
fn prompt(ui_language: &str) -> String {
    format!(
        "This image is a crop of a video game screen. Look ONLY at the shapes of the letters \
         of the in-game text (menu items, dialogue, labels). Ignore the logo, the art and the \
         game's theme: games often use plain fonts for their text.\n\
         First classify the letters: serif or sans-serif; geometric, grotesque, humanist, \
         slab or decorative; stroke contrast and weight; letter case: normal, all uppercase, \
         or small caps (lowercase letters drawn as smaller capitals).\n\
         Then name the {ASKED} font families available on Google Fonts (fonts.google.com) \
         whose letters are closest, best match first. Use exact Google Fonts family names; \
         never name a font that isn't on Google Fonts.\n\
         Answer with JSON only, no markdown: \
         {{\"text\":\"...\",\"case\":\"normal\",\"fonts\":[{{\"family\":\"Family Name\",\"weight\":400,\"reason\":\"...\"}}]}} \
         where text is one line of the game text exactly as written (at most 60 characters), \
         case is normal, upper or smallCaps, weight is the closest weight (100 to 900) \
         and reason is a few words in {lang} about what matches.",
        lang = language_name(ui_language),
    )
}

/// Reads the model's JSON (text it read, letter case, guesses), tolerating
/// code fences or text around it.
fn parse_answer(answer: &str) -> Result<(String, String, Vec<Guess>), TranslateError> {
    let invalid = || TranslateError::Provider(format!("unexpected answer: {answer}"));
    let start = answer.find('{').ok_or_else(invalid)?;
    let end = answer.rfind('}').ok_or_else(invalid)?;
    let parsed: Answer = serde_json::from_str(&answer[start..=end]).map_err(|_| invalid())?;
    let guesses = parsed
        .fonts
        .into_iter()
        .filter(|g| safe_family(&g.family))
        .take(ASKED)
        .collect();
    let caps = match parsed.case.as_str() {
        "upper" | "uppercase" => "upper",
        "smallCaps" | "small caps" | "small-caps" => "smallCaps",
        _ => "",
    };
    Ok((parsed.text.chars().take(60).collect(), caps.to_string(), guesses))
}

/// Family names are letters, digits and spaces; anything else could change the URL.
fn safe_family(family: &str) -> bool {
    !family.trim().is_empty() && family.chars().all(|c| c.is_alphanumeric() || c == ' ')
}

/// Downloads a family from Google Fonts at the closest weight it has: first the
/// weight the model chose, then the regular one. Returns the file name, the
/// weight that was found and the bytes; None when the family doesn't exist.
async fn download(http: &reqwest::Client, family: &str, weight: u16) -> Option<(String, u16, Vec<u8>)> {
    let weight = weight.clamp(100, 900) / 100 * 100;
    let mut tries = vec![weight];
    if weight != 400 {
        tries.push(400);
    }
    let name = family.trim().replace(' ', "+");
    for w in tries {
        let url = format!("{CSS_API}{name}:wght@{w}");
        let Ok(response) = http.get(&url).timeout(TIMEOUT).send().await else { continue };
        if !response.status().is_success() {
            continue; // unknown family, or no such weight
        }
        let Ok(css) = response.text().await else { continue };
        let Some(font_url) = font_url(&css) else { continue };
        let Ok(font) = http.get(font_url).timeout(TIMEOUT).send().await else { continue };
        let Ok(bytes) = font.bytes().await else { continue };
        let extension = if font_url.ends_with(".woff2") { "woff2" } else { "ttf" };
        let file = format!("{}-{w}.{extension}", family.trim().replace(' ', ""));
        return Some((file, w, bytes.to_vec()));
    }
    None
}

/// The font file address in a Google Fonts stylesheet. Modern browsers get one
/// block per alphabet ("/* latin */", "/* cyrillic */"...): the latin one
/// covers Portuguese and Spanish accents. Other clients get a single file.
fn font_url(css: &str) -> Option<&str> {
    let from = css.find("/* latin */").unwrap_or(0);
    let rest = &css[from..];
    let start = rest.find("url(")? + "url(".len();
    let end = start + rest[start..].find(')')?;
    Some(rest[start..end].trim_matches(['\'', '"']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_json_inside_code_fences() {
        let answer = "```json\n{\"text\":\"Hello\",\"case\":\"smallCaps\",\"fonts\":[{\"family\":\"Cinzel\",\"weight\":600,\"reason\":\"serifas\"},{\"family\":\"Spectral\"}]}\n```";
        let (text, caps, guesses) = parse_answer(answer).unwrap();
        assert_eq!(text, "Hello");
        assert_eq!(caps, "smallCaps");
        assert_eq!(guesses.len(), 2);
        assert_eq!(guesses[0].weight, 600);
        assert_eq!(guesses[1].weight, 400); // default when missing
    }

    #[test]
    fn drops_names_that_could_change_the_url() {
        let answer = r#"{"fonts":[{"family":"Roboto&x=1"},{"family":"Open Sans"}]}"#;
        let (_, caps, guesses) = parse_answer(answer).unwrap();
        assert_eq!(caps, ""); // no case given: keep the translation as it is
        assert_eq!(guesses.len(), 1);
        assert_eq!(guesses[0].family, "Open Sans");
    }

    #[test]
    fn rejects_an_answer_without_json() {
        assert!(parse_answer("I can't tell.").is_err());
    }

    #[test]
    fn picks_the_latin_file() {
        let css = "/* cyrillic */\n@font-face { src: url(https://x/c.woff2) format('woff2'); }\n\
                   /* latin */\n@font-face { src: url(https://x/l.woff2) format('woff2'); }";
        assert_eq!(font_url(css), Some("https://x/l.woff2"));
    }

    #[test]
    fn picks_the_single_file() {
        let css = "@font-face { font-family: 'Cinzel'; src: url(https://x/cinzel.ttf) format('truetype'); }";
        assert_eq!(font_url(css), Some("https://x/cinzel.ttf"));
    }
}
