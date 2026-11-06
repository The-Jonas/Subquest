<p align="center">
  <img src="app/src-tauri/app-icon.png" width="96" alt="Subquest icon" />
</p>

# Subquest: Real-Time Game Translator

Play any game in your language. Subquest reads the dialogue on your screen, translates it with context-aware AI and shows the translation over the game, without stealing focus.

> 🇧🇷 [Leia em português](README.pt-BR.md)

**Status:** first test version. Capture, OCR, two-layer translation, the in-game overlay with the game's own fonts, the in-game toolbar and shortcuts work end to end; see the [Roadmap](#roadmap) for what's next.

## Install

1. Download `Subquest_x.y.z_x64-setup.exe` from the [Releases](https://github.com/The-Jonas/Subquest/releases) page and run it. Windows may warn that the publisher is unknown (the installer isn't signed yet): click **More info → Run anyway**.
2. Open Subquest. A guided tour shows each screen; you can open it again any time from the menu.
3. In **Settings → Translation**, paste an API key for the translator. [Google AI Studio](https://aistudio.google.com/apikey) gives a free Gemini key; it stays on your computer.
4. Run the game **windowed or borderless** (exclusive fullscreen blocks the capture), pick it in **Play**, mark where the text appears and start.

Requires Windows 10 (version 1903 or newer) or Windows 11. Japanese, Chinese or Korean games also need that language's pack installed in Windows (Settings → Time & language → Language).

## Why another translator?

Existing tools work, but tend to be slow, translate each line in isolation and bury the user under settings. Subquest focuses on three things:

- **Speed.** GPU window capture, OCR only when the text actually changes *and* stops changing, and a pipeline that always drops stale frames instead of queueing them.
- **Better translations.** Language models receive the previous lines and are told to fix OCR mistakes; a quick translator fills the gap while they answer.
- **A clean interface.** Three steps to start: pick the game, mark the dialogue area, start.

## Architecture

```
┌──────────────── core/ (C++ · subquest_core.dll) ─────────────────┐
│ capture (WGC) ─▶ change detector ─▶ [latest slot] ─▶ OCR thread  │
│ window list · window position                         │ callback │
└───────────────────────────────────────────────────────┼──────────┘
                                   C API (core_api.h)   │
┌───────────── app/src-tauri/ (Rust) ───────────────────▼──────────┐
│ FFI bridge · streaming translator (context, cache) · settings    │
│ overlay window that follows the game                             │
└────────────────────────────────▲─────────────────────────────────┘
                                 │ Tauri commands / events
┌──────────────── app/src/ (Svelte 5 + TypeScript) ────────────────┐
│ Play · Games · History · Settings · tour · overlay · toolbar     │
└──────────────────────────────────────────────────────────────────┘
```

| Layer | Language | Responsibility |
|---|---|---|
| `core/` | C++20 | Window capture, change detection, OCR, window position. Exposes a small C API. |
| `app/src-tauri/` | Rust | Loads the core DLL at runtime, translation, cache, settings, the overlay window. |
| `app/src/` | Svelte 5 + TS | User interface, the overlay page and translations of the interface. |

### Key ideas

- **Latest slot instead of a queue** (`core/src/pipeline/latest_slot.h`): if OCR is busy, only the newest frame waits; older ones are dropped. Latency never piles up.
- **Change detector** (`core/src/vision/change_detector.cpp`): compares grayscale thumbnails and fires once the text changed *and* stayed still for a few frames, so typewriter animations don't trigger OCR on every letter.
- **One translator for many services** (`app/src-tauri/src/translate/openai_compat.rs`): the OpenAI-compatible protocol covers OpenAI, Gemini, OpenRouter, Ollama and LM Studio by changing only the URL and model.

## Building from source (Windows)

Only needed to change the code; to use Subquest, [install it](#install).

Requirements: Visual Studio 2022 (C++ workload), CMake 3.24+, Rust (stable), Node.js 20+.

```powershell
# 1. Core (also copies the DLL to app/src-tauri/bin)
cd core
cmake --preset windows-release
cmake --build --preset release
ctest --test-dir build -C Release --output-on-failure

# 2. App
cd ../app
npm install
npm run tauri dev
```

To work only on the interface, `npm run dev` runs it in the browser with sample data; no Rust or C++ needed.

### Installer

With the core built (step 1 above, which puts the DLL in `app/src-tauri/bin`):

```powershell
cd app
npm run tauri build
```

The installer is written to `app/src-tauri/target/release/bundle/nsis/`. It carries the core DLL and installs WebView2 if the PC doesn't have it; both runtimes are linked statically, so nothing else is needed.

## Roadmap

1. ~~**MVP:** WGC capture, Windows OCR (English), area marking, LLM translation, overlay, per-stage latency.~~ Done.
2. **Asian languages:** PaddleOCR via ONNX Runtime + DirectML (Japanese, Chinese, Korean). *Not started; Windows OCR already reads them when the Windows language pack is installed. It is also the base for other systems (item 7): one OCR that works the same everywhere.*
3. **Quality:** ~~context~~, ~~persistent cache~~ (a JSON file instead of SQLite), ~~streaming~~, ~~game profiles~~, ~~a quick second translator~~, ~~text styles with the game's fonts~~. *Left: the per-game glossary.*
4. ~~**Full-screen mode:** text detection over the whole window, translation drawn in place.~~ Done.
5. **Polish:** ~~hotkeys~~, ~~in-game toolbar~~, ~~guided tour~~, ~~themes~~, installer *(first build)*, text hooking for supported engines.
6. **Other screens:** a second-screen mode for consoles and phones, and Android.
7. **Other systems.** Only the C++ core (capture, OCR, window list) and keeping the overlay on top are Windows-only; the interface and the Rust side already build anywhere. With PaddleOCR in place (item 2), each system needs its own capture and overlay:
   - **Linux**, aimed at the Steam Deck: X11 first (capture and an always-on-top overlay work without trouble); Wayland later, since capture goes through a permission prompt and an app can't place itself over another app's window.
   - **macOS**: ScreenCaptureKit for capture and the Vision framework for OCR (already reads Japanese, Chinese and Korean); needs the Screen Recording permission once.

## Contributing

Code, comments and commit messages are in English. To translate the interface, copy `app/src/lib/i18n/locales/pt-BR.json` to a new language file and register it in `app/src/lib/i18n/index.svelte.ts`.

## License

[GNU GPL v3.0](LICENSE). Third-party licenses are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
