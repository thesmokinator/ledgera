import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ReportCharts } from "./ReportCharts";
import type { ReportResult } from "../types";

vi.mock("@ant-design/charts", () => ({
  Column: ({ data }: { data: unknown[] }) => (
    <div data-testid="cashflow-column-chart" data-count={data.length} />
  ),
  Pie: () => null,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        "reports.chart_cashflow": "Cashflow by period",
        "reports.chart_cashflow_empty":
          "Cashflow by period will appear here once the selected report contains cash movements.",
        "balances.value": "Value",
      };
      return translations[key] ?? key;
    },
  }),
}));

function cashflowReport(periods: ReportResult["visualization"]["periods"]): ReportResult {
  return {
    reportType: "cf",
    interval: "-M",
    periodColumns: periods.map((period) => period.period),
    rows: [],
    visualization: {
      kind: "cashflow",
      entries: [],
      periods,
      accountLevel: 2,
    },
  };
}

describe("ReportCharts", () => {
  it("shows a placeholder for cashflow when there are no period summaries", () => {
    render(<ReportCharts data={cashflowReport([])} />);

    expect(screen.getByText("Cashflow by period")).toBeTruthy();
    expect(
      screen.getByText(
        "Cashflow by period will appear here once the selected report contains cash movements.",
      ),
    ).toBeTruthy();
    expect(screen.queryByTestId("cashflow-column-chart")).toBeNull();
  });

  it("keeps the multiperiod cashflow chart even with a single period", () => {
    render(
      <ReportCharts
        data={cashflowReport([
          {
            period: "2026-01",
            amount: 120,
            chartAmount: 120,
            chartAmountFormatted: "€120.00",
            commodity: "€",
            formatted: "€120.00",
            tint: "positive",
          },
        ])}
      />,
    );

    expect(screen.getByTestId("cashflow-column-chart").getAttribute("data-count")).toBe(
      "1",
    );
    expect(
      screen.queryByText(
        "Cashflow by period will appear here once the selected report contains cash movements.",
      ),
    ).toBeNull();
  });
});
