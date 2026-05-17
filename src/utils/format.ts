const countFormatter = new Intl.NumberFormat("en", { maximumFractionDigits: 0 });

export function formatCount(value: number): string {
  return countFormatter.format(value);
}

export function toAutocompleteOptions(values: string[]): { value: string }[] {
  return values.map((value) => ({ value }));
}

export function formatJournalName(path: string): string {
  const trimmedPath = path.trim();
  return trimmedPath.split(/[\\/]/).filter(Boolean).pop() ?? trimmedPath;
}
