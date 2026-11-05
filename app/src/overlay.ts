// Overlay window entry point: a separate page, so it loads only what it needs.
import "@fontsource-variable/atkinson-hyperlegible-next";
import { mount } from "svelte";
import Overlay from "./lib/overlay/Overlay.svelte";

const overlay = mount(Overlay, { target: document.getElementById("overlay")! });

export default overlay;
