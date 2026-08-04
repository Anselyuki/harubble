import 'overlayscrollbars/styles/overlayscrollbars.css';
import './app.css';
import { mount } from 'svelte';

if (import.meta.env.MODE === 'e2e') {
  const { installTauriMocks } = await import('$lib/e2e/installTauriMocks');
  installTauriMocks();
}

const isMiniPlayer =
  new URLSearchParams(window.location.search).get('window') === 'mini-player';
const { default: RootComponent } = isMiniPlayer
  ? await import('$lib/components/app/player/MiniPlayerWindow.svelte')
  : await import('./App.svelte');

const app = mount(RootComponent, {
  target: document.body,
});

export default app;
