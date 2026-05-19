interface VolumeStripMeasurement {
  badgeTop: number;
  trackTop: number;
}

export function getVolumeStripExtension(
  measurement: VolumeStripMeasurement
): number {
  return Math.max(0, measurement.trackTop - measurement.badgeTop);
}
