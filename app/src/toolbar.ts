// In-game toolbar entry point: a tiny page of its own (see lib/toolbar).
import "@fontsource-variable/atkinson-hyperlegible-next";
import "./app.css";
import { mount } from "svelte";
import { watchErrors } from "./lib/api";

watchErrors("toolbar"); // problems of the installed app end up in ui.log
import Toolbar from "./lib/toolbar/Toolbar.svelte";

const toolbar = mount(Toolbar, { target: document.getElementById("toolbar")! });

export default toolbar;
