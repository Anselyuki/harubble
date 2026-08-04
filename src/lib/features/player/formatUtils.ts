export function formatTime(seconds: number): string {
  if (!isFinite(seconds) || isNaN(seconds) || seconds < 0) return '0:00';
  const minute = Math.floor(seconds / 60);
  const second = Math.floor(seconds % 60);
  return `${minute}:${second.toString().padStart(2, '0')}`;
}

export function formatSampleRate(sampleRate: number): string {
  if (!Number.isFinite(sampleRate) || sampleRate <= 0) return '--';
  if (sampleRate % 1000 === 0) return `${sampleRate / 1000}k`;
  return `${Math.round((sampleRate / 1000) * 10) / 10}k`;
}

export function formatBitDepth(
  bitsPerSample: number | null | undefined
): string {
  if (!bitsPerSample || !Number.isFinite(bitsPerSample) || bitsPerSample <= 0) {
    return '--bit';
  }
  return `${bitsPerSample}bit`;
}

export function formatBitrate(bitrateKbps: number | null | undefined): string {
  if (!bitrateKbps || !Number.isFinite(bitrateKbps) || bitrateKbps <= 0) {
    return '';
  }
  return `${Math.round(bitrateKbps)}kbps`;
}

export function formatChannels(channels: number): string {
  if (!Number.isFinite(channels) || channels <= 0) return '--';
  return `${channels}ch`;
}

export function formatPlaybackCore(
  sampleRate: number,
  bitsPerSample: number | null | undefined
): string {
  return `${formatSampleRate(sampleRate)}/${formatBitDepth(bitsPerSample)}`;
}

export function formatPlaybackEndpoint(
  sampleRate: number,
  bitsPerSample: number | null | undefined,
  channels: number,
  bitrateKbps?: number | null
): string {
  return [
    formatPlaybackCore(sampleRate, bitsPerSample),
    formatChannels(channels),
    formatBitrate(bitrateKbps),
  ]
    .filter(Boolean)
    .join('/');
}

export function normalizeSampleFormat(sampleFormat: string): string {
  return sampleFormat.trim().toLowerCase();
}
