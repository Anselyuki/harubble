export function shouldAcceptThemeSnapshot(
  currentRevision: number,
  currentActivePackageId: string | null,
  incomingRevision: number,
  incomingActivePackageId: string | null
): boolean {
  if (incomingRevision < currentRevision) return false;
  if (
    incomingRevision === currentRevision &&
    incomingActivePackageId !== currentActivePackageId
  ) {
    return false;
  }
  return true;
}
