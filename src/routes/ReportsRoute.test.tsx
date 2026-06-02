import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ReportsRoute } from "./ReportsRoute";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@ant-design/charts", () => ({
  Bar: () => null,
  Column: () => null,
  Pie: () => null,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        "reports.income_statement": "Income Statement",
        "reports.balance_sheet": "Balance Sheet",
        "reports.cashflow": "Cashflow",
        "reports.scope": "Scope",
        "reports.scope_current_month": "Current month",
        "reports.scope_current_year": "Current year",
        "reports.scope_all_time": "All time",
        "reports.scope_custom": "Custom range",
        "reports.begin_date": "Start date",
        "reports.end_date": "End date",
        "reports.grouping": "Grouping",
        "reports.grouping_none": "No grouping",
        "reports.grouping_month": "By month",
        "reports.grouping_quarter": "By quarter",
        "reports.grouping_year": "By year",
        "reports.generate": "Generate",
        "reports.generating": "Generating report…",
        "reports.select_and_generate":
          "Select a report and interval, then click Generate.",
        "reports.generation_failed": "Unable to generate the report.",
        "transactions.account": "Account",
        "balances.value": "Value",
        "reports.detailed_table": "Detailed hledger table",
      };
      return translations[key] ?? key;
    },
    i18n: { changeLanguage: vi.fn(), resolvedLanguage: "en" },
  }),
}));

function renderWithProviders(props?: React.ComponentProps<typeof ReportsRoute>) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ReportsRoute {...props} />
    </QueryClientProvider>,
  );
}

const mockedInvoke = invoke as ReturnType<typeof vi.fn>;

const emptyVisualization = {
  kind: "breakdown",
  entries: [],
  periods: [],
  accountLevel: 2,
};

describe("ReportsRoute", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the generate button", () => {
    renderWithProviders();
    expect(screen.getByRole("button", { name: /generate/i })).toBeTruthy();
  });

  it("shows the initial empty state", () => {
    renderWithProviders();
    expect(
      screen.getByText(
        "Select a report and interval, then click Generate.",
      ),
    ).toBeTruthy();
  });

  it('shows "Income Statement" as default report type', () => {
    renderWithProviders();
    expect(screen.getByTitle("Income Statement")).toBeTruthy();
  });

  it('shows "Current month" as default scope', () => {
    renderWithProviders();
    expect(screen.getByTitle("Current month")).toBeTruthy();
  });

  it('shows "No grouping" as default grouping', () => {
    renderWithProviders();
    expect(screen.getByTitle("No grouping")).toBeTruthy();
  });

  it("calls run_report when generate is clicked", async () => {
    mockedInvoke.mockResolvedValueOnce({
      reportType: "is",
      interval: "",
      periodColumns: ["2026-01", "2026-02"],
      rows: [],
      visualization: emptyVisualization,
    });

    renderWithProviders();
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("run_report", {
        reportType: "is",
        interval: "",
        scope: "current_month",
        beginDate: null,
        endDate: null,
      });
    });
  });

  it("hides the detailed table by default", async () => {
    mockedInvoke.mockResolvedValueOnce({
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
              tint: "positive" as const,
            },
            {
              period: "2026-02",
              amount: 1200,
              commodity: "EUR",
              formatted: "1.200,00",
              tint: "positive" as const,
            },
          ],
          total: {
            period: "",
            amount: 2200,
            commodity: "EUR",
            formatted: "2.200,00",
            tint: "positive" as const,
          },
        },
      ],
      visualization: {
        kind: "breakdown",
        entries: [
          {
            account: "income:revenues",
            label: "revenues",
            amount: 2200,
            chartAmount: 2200,
            chartAmountFormatted: "2.200,00",
            commodity: "EUR",
            formatted: "2.200,00",
            tint: "positive" as const,
          },
        ],
        periods: [],
        accountLevel: 2,
      },
    });

    renderWithProviders();
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));

    await waitFor(() => {
      expect(screen.getByText("income:revenues")).toBeTruthy();
    });

    expect(screen.queryByText("Detailed hledger table")).toBeNull();
  });

  it("shows the detailed table expanded when enabled", async () => {
    mockedInvoke.mockResolvedValueOnce({
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
              tint: "positive" as const,
            },
            {
              period: "2026-02",
              amount: 1200,
              commodity: "EUR",
              formatted: "1.200,00",
              tint: "positive" as const,
            },
          ],
          total: {
            period: "",
            amount: 2200,
            commodity: "EUR",
            formatted: "2.200,00",
            tint: "positive" as const,
          },
        },
      ],
      visualization: {
        kind: "breakdown",
        entries: [
          {
            account: "income:revenues",
            label: "revenues",
            amount: 2200,
            chartAmount: 2200,
            chartAmountFormatted: "2.200,00",
            commodity: "EUR",
            formatted: "2.200,00",
            tint: "positive" as const,
          },
        ],
        periods: [],
        accountLevel: 2,
      },
    });

    renderWithProviders({ showDetailedTable: true });
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));

    await waitFor(() => {
      expect(screen.getByText("Detailed hledger table")).toBeTruthy();
    });

    expect(screen.getAllByText("Account").length).toBeGreaterThan(0);
    expect(screen.getByText("Revenues")).toBeTruthy();
  });

  it("shows error state when generation fails", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("fail"));

    renderWithProviders();
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));

    await waitFor(() => {
      expect(
        screen.getByText("Unable to generate the report."),
      ).toBeTruthy();
    });
  });

  it("shows spinner while loading", async () => {
    mockedInvoke.mockImplementation(
      () =>
        new Promise((resolve) => {
          setTimeout(() => {
            resolve({
              reportType: "is",
              interval: "-M",
              periodColumns: [],
              rows: [],
              visualization: emptyVisualization,
            });
          }, 100);
        }),
    );

    renderWithProviders();
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));

    await waitFor(() => {
      expect(screen.getByText("Generating report…")).toBeTruthy();
    });
  });
});
