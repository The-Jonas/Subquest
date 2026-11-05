<p align="center">
  <img src="app/src-tauri/app-icon.png" width="96" alt="Subquest icon" />
</p>

# Subquest: Real-Time Game Translator

Play any game in your language. Subquest reads the dialogue on your screen, translates it with context-aware AI and shows the translation over the game, without stealing focus.

> 🇧🇷 [Leia em português](README.pt-BR.md)

**Status:** early development. The interface and the architecture are in place; screen capture, OCR and translation are being built (see [Roadmap](#roadmap)).

## Why another translator?

Existing tools work, but tend to be slow, translate each line in isolation and bury the user under settings. Subquest focuses on three things:

- **Speed.** GPU window capture, OCR only when the text actually changes *and* stops changing, and a pipeline that always drops stale frames instead of queueing them.
- **Better translations.** Language models receive the previous lines and a per-game glossary, and are told to fix OCR mistakes.
- **A clean interface.** Three steps to start: pick the game, mark the dialogue area, start.

## Architecture

```
┌──────────────── core/ (C++ · subquest_core.dll) ────────────────┐
│ capture (WGC) ─▶ change detector ─▶ [latest slot] ─▶ OCR thread │
│                                                    │           │
│ overlay window (Direct2D) ◀── sq_overlay_set_text  │ callback  │
└────────────────────────────────▲───────────────────┼───────────┘
                                 │      C API (core_api.h)
┌───────────── app/src-tauri/ (Rust) ────────────────▼───────────┐
│ FFI bridge · translators (cache, context, glossary) · settings │
└────────────────────────────────▲───────────────────────────────┘
                                 │ Tauri commands / events
┌──────────────── app/src/ (Svelte 5 + TypeScript) ──────────────┐
│ Play · Profiles · Settings · Tutorial · i18n (pt-BR, en, es)   │
└────────────────────────────────────────────────────────────────┘
```

| Layer | Language | Responsibility |
|---|---|---|
| `core/` | C++20 | Window capture, change detection, OCR, overlay window. Exposes a small C API. |
| `app/src-tauri/` | Rust | Loads the core DLL at runtime, translation backends, cache, settings. |
| `app/src/` | Svelte 5 + TS | User interface and translations of the interface. |

### Key ideas

- **Latest slot instead of a queue** (`core/src/pipeline/latest_slot.h`): if OCR is busy, only the newest frame waits; older ones are dropped. Latency never piles up.
- **Change detector** (`core/src/vision/change_detector.cpp`): compares grayscale thumbnails and fires once the text changed *and* stayed still for a few frames, so typewriter animations don't trigger OCR on every letter.
- **One translator for many services** (`app/src-tauri/src/translate/openai_compat.rs`): the OpenAI-compatible protocol covers OpenAI, Gemini, OpenRouter, Ollama and LM Studio by changing only the URL and model.

## Building (Windows)

Requirements: Visual Studio 2022 (C++ workload), CMake 3.24+, Rust (stable), Node.js 20+.

```powershell
# 1. Core (also copies the DLL to app/src-tauri/bin)
cd core
cmake --preset windows-release
cmake --build --preset release
ctest --preset release

# 2. App
cd ../app
npm install
npm run tauri dev
```

To work only on the interface, `npm run dev` runs it in the browser with sample data; no Rust or C++ needed.

## Roadmap

1. **MVP:** WGC capture, Windows OCR (English), area marking, LLM translation, overlay, per-stage latency.
2. **Asian languages:** PaddleOCR via ONNX Runtime + DirectML (Japanese, Chinese, Korean).
3. **Quality:** context, glossary, persistent cache (SQLite), streaming, game profiles.
4. **Full-screen mode:** text detection over the whole window, translation drawn in place.
5. **Polish:** hotkeys, installer, text hooking for supported engines.

## Contributing

Code identifiers and file names are in English; code comments are in Portuguese. To translate the interface, copy `app/src/lib/i18n/locales/pt-BR.json` to a new language file and register it in `app/src/lib/i18n/index.svelte.ts`.

## License

[GNU GPL v3.0](LICENSE). Third-party licenses are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
