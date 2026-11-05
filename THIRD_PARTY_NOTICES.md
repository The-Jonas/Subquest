# Third-party notices

Subquest is licensed under the GNU GPL v3.0. It depends on the projects below,
each under its own license. Keep this file updated whenever a dependency is added.

| Project | Used for | License |
|---|---|---|
| [Tauri](https://github.com/tauri-apps/tauri) | Desktop shell (Rust) | MIT / Apache-2.0 |
| [Svelte](https://github.com/sveltejs/svelte) | User interface | MIT |
| [Vite](https://github.com/vitejs/vite) | Frontend build | MIT |
| [libloading](https://github.com/nagisa/rust_libloading) | Loading `subquest_core.dll` | ISC |
| [serde](https://github.com/serde-rs/serde) | Config (de)serialization | MIT / Apache-2.0 |
| [reqwest](https://github.com/seanmonstar/reqwest) | Translation HTTP requests (streaming) | MIT / Apache-2.0 |
| [futures-util](https://github.com/rust-lang/futures-rs) | Reading the response stream | MIT / Apache-2.0 |
| [tokio](https://github.com/tokio-rs/tokio) | Pauses between retries, sharing a translation between OCR re-readings | MIT |
| [base64](https://github.com/marshallpierce/rust-base64) | Sending the game crop to the model (font detection) | MIT / Apache-2.0 |
| [Bricolage Grotesque](https://github.com/ateliertriay/bricolage) | UI typeface | SIL OFL 1.1 |
| [Atkinson Hyperlegible Next](https://www.brailleinstitute.org/freefont/) | UI typeface | SIL OFL 1.1 |
| [Cinzel](https://github.com/NDISCOVER/Cinzel) | Overlay font option | SIL OFL 1.1 |
| [Crimson Pro](https://github.com/Fonthausen/CrimsonPro) | Overlay font option | SIL OFL 1.1 |
| [Pixelify Sans](https://github.com/eifetx/Pixelify-Sans) | Overlay font option | SIL OFL 1.1 |

Fonts found by "Detect font" are downloaded from [Google Fonts](https://fonts.google.com) on the user's request, under their own open licenses (mostly SIL OFL 1.1). They are saved in the user's app data folder and are not shipped with Subquest.

Planned (not yet included):

| Project | Used for | License |
|---|---|---|
| [ONNX Runtime](https://github.com/microsoft/onnxruntime) | Running OCR models on GPU (DirectML) | MIT |
| [PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) models | Japanese / Chinese / Korean OCR | Apache-2.0 |
