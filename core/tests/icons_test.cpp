// Test for icons (pictures inside a line of text), no framework: a gamepad
// button between two words, or read as a word, becomes a "{n}" placeholder;
// plain text is left alone.
#include <cstdio>

#include "vision/icons.h"

using namespace subquest;

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

static const uint32_t BG = 0x202020, TEXT = 0xF0F0F0, GREEN = 0x30B040;

// A 400 x 60 frame of background color.
static Frame blank() {
    Frame f;
    f.width = 400;
    f.height = 60;
    f.stride = f.width * 4;
    f.bgra.assign(size_t(f.stride) * f.height, 0x20);
    for (size_t i = 3; i < f.bgra.size(); i += 4) f.bgra[i] = 255;
    return f;
}

// Paints a box of a color (0xRRGGBB).
static void paint(Frame& f, Rect r, uint32_t rgb) {
    for (int y = r.y; y < r.y + r.height; ++y)
        for (int x = r.x; x < r.x + r.width; ++x) {
            uint8_t* px = f.bgra.data() + size_t(y) * f.stride + size_t(x) * 4;
            px[0] = rgb & 0xFF;
            px[1] = (rgb >> 8) & 0xFF;
            px[2] = (rgb >> 16) & 0xFF;
        }
}

// "Letters" of a word: thin vertical strokes of the text color in its box.
static void letters(Frame& f, Rect r) {
    for (int x = r.x; x < r.x + r.width; x += 5) paint(f, {x, r.y, 2, r.height}, TEXT); // 40% ink, like bold letters
}

static OcrLine make_line(std::vector<OcrWord> words) {
    OcrLine line;
    line.text_rgb = TEXT;
    line.background_rgb = BG;
    line.words = std::move(words);
    int x0 = line.words.front().box.x, x1 = line.words.back().box.x + line.words.back().box.width;
    line.box = {x0, 20, x1 - x0, 16};
    for (const OcrWord& w : line.words) line.text += (line.text.empty() ? "" : " ") + w.text;
    return line;
}

int main() {
    {
        // "Press [button] to skip": OCR skipped the button, leaving a wide gap.
        Frame f = blank();
        const Rect press{20, 20, 60, 16}, to{120, 20, 20, 16}, skip{146, 20, 40, 16};
        letters(f, press), letters(f, to), letters(f, skip);
        paint(f, {88, 17, 22, 22}, GREEN); // the button, a bit taller than the letters
        paint(f, {96, 22, 6, 12}, TEXT);   // with a white "A" on it
        std::vector<OcrLine> lines{make_line({{"Press", press}, {"to", to}, {"skip", skip}})};
        const std::vector<Rect> icons = find_icons(f, lines);
        check(icons.size() == 1, "a button in a gap between words is found");
        check(lines[0].text == "Press {1} to skip", "and its place in the text gets a placeholder");
        check(!icons.empty() && icons[0].x == 88 && icons[0].width == 22 && icons[0].height == 22,
              "its box is the drawn button, not the whole gap");
        check(lines[0].words.size() == 4 && lines[0].words[1].text == "{1}", "the words follow");
    }
    {
        // The button read as a word ("A"): mostly green, so a picture.
        Frame f = blank();
        const Rect press{20, 20, 60, 16}, a{86, 17, 22, 22}, jump{114, 20, 40, 16};
        letters(f, press), letters(f, jump);
        paint(f, a, GREEN);
        paint(f, {94, 22, 6, 12}, TEXT);
        std::vector<OcrLine> lines{make_line({{"Press", press}, {"A", a}, {"jump", jump}})};
        const std::vector<Rect> icons = find_icons(f, lines);
        check(icons.size() == 1 && lines[0].text == "Press {1} jump", "a button read as a letter becomes a placeholder");
    }
    {
        // Two pictures in one reading are numbered in order, across lines.
        Frame f = blank();
        const Rect a{20, 17, 22, 22}, b{80, 17, 22, 22}, go{46, 20, 26, 16}, back{106, 20, 40, 16};
        paint(f, a, GREEN), paint(f, b, 0xC03030);
        letters(f, go), letters(f, back);
        std::vector<OcrLine> lines{make_line({{"A", a}, {"go", go}}), make_line({{"B", b}, {"back", back}})};
        find_icons(f, lines);
        check(lines[0].text == "{1} go" && lines[1].text == "{2} back", "pictures are numbered across the reading");
    }
    {
        // Hades II: "In 9 [clock], grow": a dark green clock ringed in light
        // green, read as "C". Its body is nearly as dark as the background, so
        // only its colorfulness tells it from a white letter.
        Frame f = blank();
        const Rect in{20, 20, 24, 16}, nine{52, 20, 10, 16}, c{68, 17, 22, 22}, grow{98, 20, 50, 16};
        letters(f, in), letters(f, nine), letters(f, grow);
        paint(f, c, 0x75D39F);
        paint(f, {c.x + 2, c.y + 2, c.width - 4, c.height - 4}, 0x3D5342);
        paint(f, {78, 21, 2, 9}, 0x75D39F); // a hand
        std::vector<OcrLine> lines{make_line({{"In", in}, {"9", nine}, {"C", c}, {"grow", grow}})};
        const std::vector<Rect> icons = find_icons(f, lines);
        check(icons.size() == 1 && lines[0].text == "In 9 {1} grow", "a dark colored clock read as a letter is a picture");
    }
    {
        // The same clock as Hades II draws it on a dark scroll: a light green rim in
        // separate arcs around a face as dark as the background. The picture
        // is the whole clock, not its biggest arc.
        Frame f = blank();
        const Rect in{20, 20, 24, 16}, c{68, 18, 20, 20}, grow{98, 20, 50, 16};
        letters(f, in), letters(f, grow);
        paint(f, {70, 34, 16, 3}, 0x75D39F);  // bottom arc (the biggest)
        paint(f, {70, 19, 6, 3}, 0x75D39F);   // top-left arc
        paint(f, {80, 19, 6, 3}, 0x75D39F);   // top-right arc
        paint(f, {68, 23, 3, 9}, 0x75D39F);   // left side
        paint(f, {77, 24, 2, 8}, 0x75D39F); // the green hand
        std::vector<OcrLine> lines{make_line({{"In", in}, {"C", c}, {"grow", grow}})};
        const std::vector<Rect> icons = find_icons(f, lines);
        check(icons.size() == 1 && icons[0].y <= 19 && icons[0].y + icons[0].height >= 37,
              "the whole clock is the picture, top arcs included");
    }
    {
        // Hades II: "Found using the [pickaxe] Crescent Pick": a pickaxe drawn
        // in the text's own light colors, alone in the gap OCR left.
        Frame f = blank();
        const Rect the{20, 20, 30, 16}, crescent{90, 20, 70, 16};
        letters(f, the), letters(f, crescent);
        paint(f, {62, 18, 16, 3}, TEXT); // the head
        paint(f, {69, 18, 3, 20}, TEXT); // the handle
        std::vector<OcrLine> lines{make_line({{"the", the}, {"Crescent", crescent}})};
        const std::vector<Rect> icons = find_icons(f, lines);
        check(icons.size() == 1 && lines[0].text == "the {1} Crescent", "a picture in the text's colors alone in a gap is found");
    }
    {
        // Hades II flavor text: dim purple italics, "Shed for a variety". The
        // letters' cores are brighter than the line's sampled color (it
        // averages in their soft edges): the lone "a" is a word, not a picture.
        Frame f = blank();
        const uint32_t bg = 0x19182A, ink = 0x493851, core = 0x795878;
        paint(f, {0, 0, f.width, f.height}, bg);
        const Rect for_{20, 20, 30, 16}, a{56, 24, 10, 12}, variety{72, 20, 60, 16};
        for (const Rect& r : {for_, variety})
            for (int x = r.x; x < r.x + r.width; x += 5) paint(f, {x, r.y, 2, r.height}, core);
        paint(f, {56, 24, 10, 2}, core), paint(f, {56, 34, 10, 2}, core), paint(f, {64, 24, 2, 12}, core);
        paint(f, {56, 28, 2, 6}, ink);
        paint(f, {58, 26, 6, 8}, core); // bold: the "a" fills most of its tight box, like a key's face
        OcrLine line = make_line({{"for", for_}, {"a", a}, {"variety", variety}});
        line.text_rgb = ink;
        line.background_rgb = bg;
        std::vector<OcrLine> lines{line};
        check(find_icons(f, lines).empty() && lines[0].text == "for a variety", "a dim letter brighter than its line's color stays text");
    }
    {
        // Plain text, with normal spaces: nothing changes.
        Frame f = blank();
        const Rect hello{20, 20, 50, 16}, there{78, 20, 50, 16};
        letters(f, hello), letters(f, there);
        std::vector<OcrLine> lines{make_line({{"Hello", hello}, {"there", there}})};
        check(find_icons(f, lines).empty() && lines[0].text == "Hello there", "plain text is left alone");
    }
    {
        // A wide gap with nothing in it (a tab, a column) is not a picture.
        Frame f = blank();
        const Rect name{20, 20, 50, 16}, value{200, 20, 30, 16};
        letters(f, name), letters(f, value);
        std::vector<OcrLine> lines{make_line({{"Gold", name}, {"120", value}})};
        check(find_icons(f, lines).empty(), "an empty gap is just space");
    }
    {
        // A word highlighted in another color ("use a Fireball" in red) is text to translate.
        Frame f = blank();
        const Rect use{20, 20, 30, 16}, fireball{58, 20, 70, 16};
        letters(f, use);
        for (int x = fireball.x; x < fireball.x + fireball.width; x += 4) paint(f, {x, 20, 2, 16}, 0xE04030);
        std::vector<OcrLine> lines{make_line({{"use", use}, {"Fireball", fireball}})};
        check(find_icons(f, lines).empty() && lines[0].text == "use Fireball", "a highlighted word stays text");
    }
    {
        // Song lyrics in Hades II: a note before and after the words, not read by OCR.
        Frame f = blank();
        const Rect dawn{60, 20, 50, 16}, arrives{118, 20, 70, 16};
        letters(f, dawn), letters(f, arrives);
        paint(f, {30, 18, 12, 18}, 0xFFFFFF);  // a note on the left
        paint(f, {200, 18, 12, 18}, 0xFFFFFF); // and on the right
        std::vector<OcrLine> lines{make_line({{"Dawn", dawn}, {"arrives", arrives}})};
        const std::vector<Rect> icons = find_icons(f, lines);
        check(icons.size() == 2 && lines[0].text == "{1} Dawn arrives {2}", "pictures at both ends of a line are found");
        check(lines[0].box.x == 60 && lines[0].box.x + lines[0].box.width == 188,
              "and the line's box keeps only the letters (their size is measured on it)");
    }
    {
        // Hades II's "[G] GIFT": the key read as a letter, its white face filling the box.
        Frame f = blank();
        const Rect key{20, 16, 22, 24}, gift{50, 20, 60, 16};
        paint(f, key, 0xE8E8E8);
        paint(f, {28, 22, 6, 12}, 0x303030); // the dark letter on the key
        letters(f, gift);
        std::vector<OcrLine> lines{make_line({{"G", key}, {"GIFT", gift}})};
        check(find_icons(f, lines).size() == 1 && lines[0].text == "{1} GIFT", "a key read as a letter is a picture");
        check(lines[0].box.y == 20 && lines[0].box.height == 16 && lines[0].box.x == 50,
              "and the line's box loses the key (taller than the letters, it made the translation huge)");
    }
    {
        // A portrait next to the dialogue reaches past the search: not a picture of the line.
        Frame f = blank();
        const Rect hello{80, 20, 50, 16};
        letters(f, hello);
        paint(f, {0, 0, 60, 60}, 0x806040); // the portrait, from the top of the box to the bottom
        std::vector<OcrLine> lines{make_line({{"Hello", hello}})};
        check(find_icons(f, lines).empty(), "a portrait beside the text is not taken for an icon");
    }
    {
        // Two lines of a menu one above the other: the other line is not an edge picture.
        Frame f = blank();
        const Rect play{20, 20, 40, 16}, quit{70, 20, 40, 16};
        letters(f, play), letters(f, quit);
        OcrLine a = make_line({{"Play", play}});
        OcrLine b = make_line({{"Quit", quit}});
        std::vector<OcrLine> lines{a, b};
        check(find_icons(f, lines).empty(), "a neighboring line is not an icon");
    }
    {
        // Hades II lyrics: OCR read the white note as a letter ("J") glued to the green line.
        Frame f = blank();
        const Rect note{20, 18, 12, 20}, mares{40, 20, 60, 16};
        for (int x = mares.x; x < mares.x + mares.width; x += 5) paint(f, {x, 20, 2, 16}, 0x28E628);
        paint(f, {28, 18, 3, 16}, 0xFFFFFF); // the note's stem...
        paint(f, {20, 30, 9, 7}, 0xFFFFFF);  // ...and its head: thin, a quarter of its box
        OcrLine line = make_line({{"J", note}, {"Mares", mares}});
        line.text_rgb = 0x28E628;
        std::vector<OcrLine> lines{line};
        check(find_icons(f, lines).size() == 1 && lines[0].text == "{1} Mares", "a white note read as a letter in a green line is a picture");
    }
    {
        // A one-letter word ("I") with soft edges stays a word: its edges are a mix of text and background.
        Frame f = blank();
        const Rect i{20, 20, 6, 16}, know{32, 20, 40, 16};
        paint(f, {21, 20, 4, 16}, TEXT);
        paint(f, {20, 20, 1, 16}, 0x888888); // anti-aliased edges: half text, half background
        paint(f, {25, 20, 1, 16}, 0x888888);
        letters(f, know);
        std::vector<OcrLine> lines{make_line({{"I", i}, {"know", know}})};
        check(find_icons(f, lines).empty() && lines[0].text == "I know", "a short word with soft edges stays a word");
    }
    {
        // Hades II lyrics on a thin black strip: the search reaches the scenery above
        // and below, which must not hide the note standing on its own.
        Frame f = blank();
        paint(f, {0, 0, 400, 14}, 0x704080);  // scenery above the strip
        paint(f, {0, 42, 400, 18}, 0x506030); // and below
        const Rect sides{60, 20, 50, 16}, coin{118, 20, 40, 16};
        letters(f, sides), letters(f, coin);
        paint(f, {36, 19, 12, 18}, 0xFFFFFF); // the note before the words
        std::vector<OcrLine> lines{make_line({{"Sides", sides}, {"coin", coin}})};
        check(find_icons(f, lines).size() == 1 && lines[0].text == "{1} Sides coin",
              "a note on a thin strip is found, scenery around or not");
    }
    {
        // "Right on time." in Hades II: "on" stressed in white bold inside a green line is a word.
        Frame f = blank();
        const Rect right{20, 20, 50, 16}, on{78, 17, 26, 21}, time{112, 20, 40, 16};
        for (const Rect& r : {right, time})
            for (int x = r.x; x < r.x + r.width; x += 5) paint(f, {x, r.y, 2, r.height}, 0x28E628);
        for (int x = on.x; x < on.x + on.width; x += 5) paint(f, {x, on.y, 3, on.height}, 0xFFFFFF);
        OcrLine line = make_line({{"Right", right}, {"on", on}, {"time.", time}});
        line.text_rgb = 0x28E628;
        std::vector<OcrLine> lines{line};
        check(find_icons(f, lines).empty() && lines[0].text == "Right on time.", "a stressed two-letter word stays a word");
    }
    {
        // "[E] SHOP": OCR boxed the key together with the space and the start of the S.
        Frame f = blank();
        const Rect key{20, 16, 40, 24}, shop{50, 20, 60, 16};
        paint(f, {20, 16, 22, 24}, 0xE8E8E8); // the key's face, 22 px wide
        paint(f, {28, 22, 6, 12}, 0x303030);
        letters(f, shop);
        std::vector<OcrLine> lines{make_line({{"E", key}, {"SHOP", shop}})};
        const std::vector<Rect> icons = find_icons(f, lines);
        check(icons.size() == 1 && icons[0].x == 20 && icons[0].width == 22,
              "a key boxed with the start of the next word is cut to the key alone");
    }
    {
        // "♫Mares": OCR boxed the white note into the first word but left it out of the text.
        Frame f = blank();
        const Rect mares{20, 20, 80, 16}, refletem{108, 20, 60, 16};
        paint(f, {22, 18, 3, 16}, 0xFFFFFF); // the note: stem...
        paint(f, {20, 28, 8, 7}, 0xFFFFFF);  // ...and head
        for (const Rect& r : {Rect{40, 20, 60, 16}, refletem})
            for (int x = r.x; x < r.x + r.width; x += 5) paint(f, {x, r.y, 2, r.height}, 0x28E628);
        OcrLine line = make_line({{"Mares", mares}, {"refletem", refletem}});
        line.text_rgb = 0x28E628;
        std::vector<OcrLine> lines{line};
        check(find_icons(f, lines).size() == 1 && lines[0].text == "{1} Mares refletem",
              "a note boxed into the first word is found");
        check(lines[0].box.x >= 28, "and the word's box keeps only its letters");
    }
    {
        // Hades II item cards: "· Found using the...": the purple bullet is too small to be a picture.
        Frame f = blank();
        const Rect dot{20, 26, 4, 4}, found{30, 20, 50, 16};
        paint(f, dot, 0x9070B0);
        letters(f, found);
        std::vector<OcrLine> lines{make_line({{"\xc2\xb7", dot}, {"Found", found}})};
        check(find_icons(f, lines).empty() && lines[0].text == "\xc2\xb7 Found", "a bullet stays text");
    }
    {
        // A note with a scenery speck beside it: the picture is the note alone, at its own size.
        Frame f = blank();
        const Rect mares{60, 20, 50, 16};
        letters(f, mares);
        paint(f, {36, 19, 12, 18}, 0xFFFFFF); // the note
        paint(f, {26, 24, 5, 5}, 0x707070);   // a speck of scenery, left of it
        std::vector<OcrLine> lines{make_line({{"Mares", mares}})};
        const std::vector<Rect> icons = find_icons(f, lines);
        check(icons.size() == 1 && icons[0].x == 36 && icons[0].width == 12, "the note is cut alone, without the speck");
    }
    check(icon_placeholder(0) == "{1}" && icon_placeholder(9) == "{10}", "placeholders count from 1");

    std::printf(failures ? "%d FAILURE(S)\n" : "ALL OK\n", failures);
    return failures ? 1 : 0;
}
