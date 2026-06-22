import { BarChartOutlined } from "@ant-design/icons";
import { Button, Card, DatePicker, Empty, Progress, Select, Space, Spin, Table } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { callCommand } from "../utils/command";
import type { BudgetReport, BudgetRow } from "../types";
import styles from "./ReportsRoute.module.css";

type BudgetScope = "current_month" | "current_year" | "all_time" | "custom";
type BudgetGrouping = "" | "-M" | "-Q" | "-Y";

const scopeOptions = [
  { value: "current_month" as const, labelKey: "reports.scope_current_month" },
  { value: "current_year" as const, labelKey: "reports.scope_current_year" },
  { value: "all_time" as const, labelKey: "reports.scope_all_time" },
  { value: "custom" as const, labelKey: "reports.scope_custom" },
];

const groupingOptions = [
  { value: "" as const, labelKey: "reports.grouping_none" },
  { value: "-M" as const, labelKey: "reports.grouping_month" },
  { value: "-Q" as const, labelKey: "reports.grouping_quarter" },
  { value: "-Y" as const, labelKey: "reports.grouping_year" },
];

function pctColor(pctUsed: number): string {
  if (Number.isNaN(pctUsed)) return "#1677ff";
  if (pctUsed > 100) return "#ff4d4f";
  if (pctUsed > 80) return "#faad14";
  return "#52c41a";
}

export function BudgetRoute() {
  const { t } = useTranslation();
  const [scope, setScope] = useState<BudgetScope>("current_month");
  const [grouping, setGrouping] = useState<BudgetGrouping>("-M");
  const [customBeginDate, setCustomBeginDate] = useState("");
  const [customEndDate, setCustomEndDate] = useState("");

  const budgetQuery = useQuery({
    queryKey: ["budget", scope, grouping, customBeginDate, customEndDate],
    queryFn: () => callCommand<BudgetReport>("run_budget_report", {
      interval: grouping,
      scope,
      beginDate: scope === "custom" ? customBeginDate || null : null,
      endDate: scope === "custom" ? customEndDate || null : null,
    }),
    enabled: false,
    retry: false,
  });

  const localizedScopes = useMemo(
    () => scopeOptions.map((opt) => ({ value: opt.value, label: t(opt.labelKey) })),
    [t],
  );

  const localizedGroupings = useMemo(
    () => groupingOptions.map((opt) => ({ value: opt.value, label: t(opt.labelKey) })),
    [t],
  );

  const columns = useMemo(() => {
    const cols: Parameters<typeof Table<BudgetRow>>[0]["columns"] = [
      {
        title: t("transactions.account"),
        dataIndex: "account",
        render: (account: string) => <span>{account}</span>,
      },
    ];

    const periodColumns = budgetQuery.data?.periodColumns ?? [];
    for (const period of periodColumns) {
      cols.push({
        title: period,
        align: "right" as const,
        width: 160,
        render: (_: unknown, row: BudgetRow) => {
          const entry = row.periods.find((p) => p.period === period);
          if (!entry) return "-";
          return (
            <div style={{ textAlign: "right" }}>
              <div style={{ fontSize: 13, color: entry.tint === "negative" ? "#ff4d4f" : entry.tint === "positive" ? "#52c41a" : undefined }}>
                {entry.actualFormatted} / {entry.budgetFormatted}
              </div>
              <Progress
                percent={Number.isNaN(entry.pctUsed) ? 0 : Math.min(Math.abs(entry.pctUsed), 100)}
                size="small"
                strokeColor={pctColor(entry.pctUsed)}
                showInfo={false}
                style={{ margin: 0 }}
              />
            </div>
          );
        },
      });
    }

    cols.push({
      title: t("budget.total"),
      align: "right" as const,
      width: 160,
      render: (_: unknown, row: BudgetRow) => (
        <div style={{ textAlign: "right" }}>
          <div style={{ fontSize: 13, fontWeight: 500, color: row.tint === "negative" ? "#ff4d4f" : row.tint === "positive" ? "#52c41a" : undefined }}>
            {row.totalActualFormatted} / {row.totalBudgetFormatted}
          </div>
          <Progress
            percent={Number.isNaN(row.totalPctUsed) ? 0 : Math.min(Math.abs(row.totalPctUsed), 100)}
            size="small"
            strokeColor={pctColor(row.totalPctUsed)}
            showInfo={false}
            style={{ margin: 0 }}
          />
        </div>
      ),
    });

    return cols;
  }, [budgetQuery.data?.periodColumns, t]);

  const isLoading = budgetQuery.isFetching;
  const canGenerate = scope !== "custom" || Boolean(customBeginDate && customEndDate);
  const data = budgetQuery.data?.rows ?? [];
  const hasError = budgetQuery.isError;

  return (
    <div className={styles.container}>
      <Card size="small" className={styles.controls}>
        <Space wrap>
          <Select
            aria-label={t("reports.scope")}
            value={scope}
            onChange={setScope}
            options={localizedScopes}
            style={{ minWidth: 160 }}
          />
          {scope === "custom" ? (
            <>
              <DatePicker
                format="YYYY-MM-DD"
                placeholder={t("reports.begin_date")}
                onChange={(_, dateString) => setCustomBeginDate(String(dateString))}
              />
              <DatePicker
                format="YYYY-MM-DD"
                placeholder={t("reports.end_date")}
                onChange={(_, dateString) => setCustomEndDate(String(dateString))}
              />
            </>
          ) : null}
          <Select
            aria-label={t("reports.grouping")}
            value={grouping}
            onChange={setGrouping}
            options={localizedGroupings}
            style={{ minWidth: 150 }}
          />
          <Button
            type="primary"
            icon={<BarChartOutlined />}
            loading={isLoading}
            disabled={!canGenerate}
            onClick={() => budgetQuery.refetch()}
          >
            {t("reports.generate")}
          </Button>
        </Space>
      </Card>

      <Card className={styles.report_card}>
        {isLoading ? (
          <div className={styles.center_state}>
            <Spin />
            <span>{t("reports.generating")}</span>
          </div>
        ) : hasError ? (
          <Empty
            className={styles.center_state}
            description={t("reports.generation_failed")}
          />
        ) : !budgetQuery.data ? (
          <Empty
            className={styles.center_state}
            description={t("budget.select_and_generate")}
          />
        ) : data.length === 0 ? (
          <Empty
            className={styles.center_state}
            description={t("budget.empty")}
          />
        ) : (
          <Table<BudgetRow>
            rowKey="account"
            dataSource={data}
            pagination={false}
            scroll={{ x: "max-content" }}
            columns={columns}
          />
        )}
      </Card>
    </div>
  );
}
