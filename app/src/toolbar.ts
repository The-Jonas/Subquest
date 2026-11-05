// In-game toolbar entry point: a tiny page of its own (see lib/toolbar).
import "@fontsource-variable/atkinson-hyperlegible-next";
import "./app.css";
import { mount } from "svelte";
import Toolbar from "./lib/toolbar/Toolbar.svelte";

const toolbar = mount(Toolbar, { target: document.getElementById("toolbar")! });

export default toolbar;
