const countFormatter = new Intl.NumberFormat("en", { maximumFractionDigits: 0 });

export function formatCount(value: number): string {
  return countFormatter.format(value);
}

export function toAutocompleteOptions(values: string[]): { value: string }[] {
  return values.map((value) => ({ value }));
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  return `${mb.toFixed(1)} MB`;
}
