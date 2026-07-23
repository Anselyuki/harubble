import 'overlayscrollbars/styles/overlayscrollbars.css';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import MiniPlayerWindow from '$lib/components/app/player/MiniPlayerWindow.svelte';
import './endfield.css';

const RootComponent =
  new URLSearchParams(window.location.search).get('window') === 'mini-player'
    ? MiniPlayerWindow
    : App;

const app = mount(RootComponent, {
  target: document.body,
});

export default app;
