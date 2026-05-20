const WAVE_LAYERS = [
  { att: 1, opacity: 0.6, speed: 1.0, freq: 3 },
  { att: 2, opacity: 0.4, speed: 0.8, freq: 4 },
  { att: -2, opacity: 0.3, speed: 1.2, freq: 5 },
  { att: 4, opacity: 0.2, speed: 0.6, freq: 2.5 },
] as const;

export { WAVE_LAYERS };

function getWaveColors(): string[] {
  const style = getComputedStyle(document.documentElement);
  return WAVE_LAYERS.map((_, i) => {
    const raw = style.getPropertyValue(`--wave-color-${i}`).trim();
    return raw ? `rgb(${raw})` : 'rgba(250,45,72,0.6)';
  });
}

// --- Pre-computed frame table (built at module load) ---

const TOTAL_FRAMES = 480;
const TARGET_FPS = 30;
const FRAME_INTERVAL = 1000 / TARGET_FPS;
const TWO_PI = Math.PI * 2;
const PHASE_STEP = 0.02;
const PATH_STEPS = 32;
const W = 200;
const H = 50;
const B = 2;

function siriGlobalAtt(x: number): number {
  const K = 4;
  return Math.pow(K / (K + Math.pow(Math.abs(x), K)), K);
}

function precompute(): string[][] {
  const frames: string[][] = [];

  for (let f = 0; f < TOTAL_FRAMES; f++) {
    const phase = f * PHASE_STEP;
    const amplitude = 0.45 + 0.15 * Math.sin(phase * 0.8);
    const layerPaths: string[] = [];

    for (let li = 0; li < WAVE_LAYERS.length; li++) {
      const layer = WAVE_LAYERS[li];
      const layerPhase = phase * layer.speed * TWO_PI;
      let d = '';

      for (let idx = 0; idx <= PATH_STEPS; idx++) {
        const i = (idx / PATH_STEPS) * 2 * B - B;
        const x = (idx / PATH_STEPS) * W;
        const envelope = siriGlobalAtt(i * layer.att);
        const sine = Math.abs(Math.sin(layer.freq * i - layerPhase));
        const y = H - sine * envelope * amplitude * H * 0.85;
        d += idx === 0 ? `M${x | 0},${y | 0}` : `L${x | 0},${y | 0}`;
      }

      layerPaths.push(d);
    }

    frames.push(layerPaths);
  }

  return frames;
}

const frameTable: string[][] = precompute();

// --- Controller ---

export interface WaveController {
  start(): void;
  stop(): void;
}

export function createWaveLoop(
  getGroupRef: () => SVGGElement | null
): WaveController {
  let rafId: number | null = null;
  let frameIndex = 0;
  let lastTime = 0;
  let cachedPaths: SVGPathElement[] | null = null;

  function tick(now: number) {
    rafId = requestAnimationFrame(tick);

    if (now - lastTime < FRAME_INTERVAL) return;
    lastTime = now - ((now - lastTime) % FRAME_INTERVAL);

    const groupRef = getGroupRef();
    if (!groupRef) return;

    if (!cachedPaths) {
      cachedPaths = Array.from(groupRef.querySelectorAll('path'));
      const colors = getWaveColors();
      for (let i = 0; i < cachedPaths.length; i++) {
        cachedPaths[i].setAttribute('stroke', colors[i]);
      }
    }

    const frame = frameTable[frameIndex];
    for (let i = 0; i < frame.length; i++) {
      cachedPaths[i].setAttribute('d', frame[i]);
    }

    frameIndex = (frameIndex + 1) % TOTAL_FRAMES;
  }

  return {
    start() {
      if (rafId !== null) return;
      cachedPaths = null;
      lastTime = 0;
      rafId = requestAnimationFrame(tick);
    },
    stop() {
      if (rafId !== null) {
        cancelAnimationFrame(rafId);
        rafId = null;
      }
      cachedPaths = null;
    },
  };
}
