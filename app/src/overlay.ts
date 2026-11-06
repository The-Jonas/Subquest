// Overlay window entry point: a separate page, so it loads only what it needs.
import "@fontsource-variable/atkinson-hyperlegible-next";
// Fonts offered for the overlay (see lib/fonts.ts)
import "@fontsource-variable/cinzel";
import "@fontsource-variable/crimson-pro";
import "@fontsource-variable/pixelify-sans";
// Theme colors only (app.css would paint the page, and the overlay must stay see-through)
import "./theme.css";
import { mount } from "svelte";
import Overlay from "./lib/overlay/Overlay.svelte";

const overlay = mount(Overlay, { target: document.getElementById("overlay")! });

export default overlay;
