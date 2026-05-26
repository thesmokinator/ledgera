import type { ReportResult, ReportRow } from "../types";

export interface ChartDatum {
  period: string;
  account: string;
  amount: number;
}

export interface PieDatum {
  account: string;
  amount: number;
}

/**
 * Transforms report rows into a flat array for column/bar charts.
 * Each row's period amounts become individual data points.
 * Filters out total rows and optionally limits to top-level (indent=1) accounts.
 */
export function toColumnChartData(result: ReportResult): ChartDatum[] {
  const data: ChartDatum[] = [];

  for (const row of result.rows) {
    // Skip total/subtotal rows
    if (row.isTotal) continue;

    for (const amt of row.amounts) {
      data.push({
        period: amt.period,
        account: row.account,
        amount: amt.amount,
      });
    }
  }

  return data;
}

/**
 * Transforms report rows into pie chart data using each row's total amount.
 * Filters out total rows to avoid double-counting.
 */
export function toPieChartData(rows: ReportRow[]): PieDatum[] {
  return rows
    .filter((row) => !row.isTotal)
    .map((row) => ({
      account: row.account,
      amount: row.total.amount,
    }));
}

/**
 * Picks a reasonable number of colors for charts.
 * Uses Ant Design's color palette tokens conceptually;
 * actual colors are applied via CSS or chart config.
 */
export function chartColorPalette(count: number): string[] {
  // A curated palette that works in both light and dark themes
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

  // Cycle through palette if more colors are needed
  const colors: string[] = [];
  for (let i = 0; i < count; i++) {
    colors.push(palette[i % palette.length]);
  }
  return colors;
}

/**
 * Groups chart data by account and returns unique account names
 * sorted by total absolute amount (descending).
 */
export function topAccountsByAmount(rows: ReportRow[], limit = 10): string[] {
  return rows
    .filter((row) => !row.isTotal)
    .sort((a, b) => Math.abs(b.total.amount) - Math.abs(a.total.amount))
    .slice(0, limit)
    .map((row) => row.account);
}
