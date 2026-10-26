import { mount } from "svelte";
import "@fontsource-variable/archivo/wdth.css";
import "./styles/theme.css";
import "./styles/base.css";
import "./styles/app.css";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });
