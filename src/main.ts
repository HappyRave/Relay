import { mount } from "svelte";
import "@fontsource/archivo/400.css";
import "@fontsource/archivo/600.css";
import "@fontsource/archivo/800.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/components.css";
import "./styles/app.css";
import App from "./App.svelte";

const app = mount(App, { target: document.getElementById("app")! });

export default app;
