// UI entry point: local fonts (works offline), styles and mounting.
import "@fontsource-variable/bricolage-grotesque";
import "@fontsource-variable/atkinson-hyperlegible-next";
// Fonts offered for the overlay (see lib/fonts.ts)
import "@fontsource-variable/cinzel";
import "@fontsource-variable/crimson-pro";
import "@fontsource-variable/pixelify-sans";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";

const app = mount(App, { target: document.getElementById("app")! });

export default app;
