export function chartColorPalette(count: number): string[] {
  const palette = [
    "#10b981", // emerald (primary)
    "#ef4444", // red
    "#3b82f6", // blue
    "#f59e0b", // amber
    "#8b5cf6", // violet
    "#06b6d4", // cyan
    "#f97316", // orange
    "#ec4899", // pink
    "#14b8a6", // teal
    "#a855f7", // purple
    "#84cc16", // lime
    "#e11d48", // rose
  ];

  const colors: string[] = [];
  for (let i = 0; i < count; i++) {
    colors.push(palette[i % palette.length]);
  }
  return colors;
}
