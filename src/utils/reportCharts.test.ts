import { describe, expect, it } from "vitest";
import {
  toColumnChartData,
  toPieChartData,
  chartColorPalette,
  topAccountsByAmount,
} from "./reportCharts";
import type { ReportResult } from "../types";

const sampleResult: ReportResult = {
  reportType: "is",
  interval: "-M",
  periodColumns: ["2026-01", "2026-02"],
  rows: [
    {
      account: "Revenues",
      indent: 1,
      isTotal: false,
      amounts: [
        {
          period: "2026-01",
          amount: 1000,
          commodity: "EUR",
          formatted: "1.000,00",
          tint: "positive",
        },
        {
          period: "2026-02",
          amount: 1200,
          commodity: "EUR",
          formatted: "1.200,00",
          tint: "positive",
        },
      ],
      total: {
        period: "",
        amount: 2200,
        commodity: "EUR",
        formatted: "2.200,00",
        tint: "positive",
      },
    },
    {
      account: "Expenses",
      indent: 1,
      isTotal: false,
      amounts: [
        {
          period: "2026-01",
          amount: -800,
          commodity: "EUR",
          formatted: "-800,00",
          tint: "negative",
        },
        {
          period: "2026-02",
          amount: -900,
          commodity: "EUR",
          formatted: "-900,00",
          tint: "negative",
        },
      ],
      total: {
        period: "",
        amount: -1700,
        commodity: "EUR",
        formatted: "-1.700,00",
        tint: "negative",
      },
    },
    {
      account: "Net Income",
      indent: 1,
      isTotal: true,
      amounts: [
        {
          period: "2026-01",
          amount: 200,
          commodity: "EUR",
          formatted: "200,00",
          tint: "positive",
        },
        {
          period: "2026-02",
          amount: 300,
          commodity: "EUR",
          formatted: "300,00",
          tint: "positive",
        },
      ],
      total: {
        period: "",
        amount: 500,
        commodity: "EUR",
        formatted: "500,00",
        tint: "positive",
      },
    },
  ],
};

describe("toColumnChartData", () => {
  it("flattens rows with period amounts, excluding total rows", () => {
    const result = toColumnChartData(sampleResult);

    expect(result).toHaveLength(4);
    expect(result).toContainEqual({
      period: "2026-01",
      account: "Revenues",
      amount: 1000,
      formatted: "1.000,00",
    });
    expect(result).toContainEqual({
      period: "2026-02",
      account: "Expenses",
      amount: -900,
      formatted: "-900,00",
    });
  });

  it("does not include total rows", () => {
    const result = toColumnChartData(sampleResult);
    const totalAccounts = result.filter((d) => d.account === "Net Income");
    expect(totalAccounts).toHaveLength(0);
  });

  it("returns empty array for empty rows", () => {
    const result = toColumnChartData({ ...sampleResult, rows: [] });
    expect(result).toEqual([]);
  });

  it("handles rows with no period amounts", () => {
    const emptyRowsResult: ReportResult = {
      reportType: "is",
      interval: "",
      periodColumns: [],
      rows: [
        {
          account: "Test",
          indent: 1,
          isTotal: false,
          amounts: [],
          total: {
            period: "",
            amount: 0,
            commodity: "EUR",
            formatted: "0,00",
            tint: "neutral",
          },
        },
      ],
    };
    const result = toColumnChartData(emptyRowsResult);
    expect(result).toEqual([]);
  });
});

describe("toPieChartData", () => {
  it("extracts total amounts with formatted strings from non-total rows", () => {
    const result = toPieChartData(sampleResult.rows);

    expect(result).toHaveLength(2);
    expect(result).toContainEqual({
      account: "Revenues",
      amount: 2200,
      chartAmount: 2200,
      formatted: "2.200,00",
    });
    expect(result).toContainEqual({
      account: "Expenses",
      amount: -1700,
      chartAmount: 1700,
      formatted: "-1.700,00",
    });
  });

  it("excludes total rows", () => {
    const result = toPieChartData(sampleResult.rows);
    const netIncome = result.filter((d) => d.account === "Net Income");
    expect(netIncome).toHaveLength(0);
  });
});

describe("chartColorPalette", () => {
  it("returns the requested number of colors", () => {
    expect(chartColorPalette(3)).toHaveLength(3);
    expect(chartColorPalette(20)).toHaveLength(20);
  });

  it("cycles through the base palette", () => {
    const colors = chartColorPalette(15);
    expect(colors[0]).toBe(colors[12]);
  });
});

describe("topAccountsByAmount", () => {
  it("returns account names sorted by absolute total amount descending", () => {
    const result = topAccountsByAmount(sampleResult.rows);
    expect(result).toEqual(["Revenues", "Expenses"]);
  });

  it("respects the limit parameter", () => {
    const result = topAccountsByAmount(sampleResult.rows, 1);
    expect(result).toHaveLength(1);
    expect(result[0]).toBe("Revenues");
  });

  it("excludes total rows", () => {
    const result = topAccountsByAmount(sampleResult.rows);
    expect(result).not.toContain("Net Income");
  });
});
