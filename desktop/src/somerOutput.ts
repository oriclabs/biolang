export function snapshotDelta(value: string, offset: number | undefined, cursor: number) {
  const startOffset = offset ?? 0;
  return {
    data: value.slice(Math.max(0, cursor - startOffset)),
    cursor: startOffset + value.length,
  };
}
