import { BarChartOutlined } from "@ant-design/icons";
import { Button, Card, Collapse, DatePicker, Empty, Select, Space, Spin, Table } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { callCommand } from "../utils/command";
import { Amount } from "../components/Amount";
import { ReportCharts } from "../components/ReportCharts";
import type { ReportPeriodAmount, ReportResult, ReportRow } from "./types";
import styles from "./ReportsRoute.module.css";

type ReportType = "is" | "bs" | "cf";
type ReportScope = "current_month" | "current_year" | "all_time" | "custom";
type ReportGrouping = "" | "-M" | "-Q" | "-Y";

const reportTypeOptions = [
  { value: "is" as const, labelKey: "reports.income_statement" },
  { value: "bs" as const, labelKey: "reports.balance_sheet" },
  { value: "cf" as const, labelKey: "reports.cashflow" },
];

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

export function ReportsRoute({
  showDetailedTable = false,
}: {
  showDetailedTable?: boolean;
}) {
  const { t } = useTranslation();
  const [reportType, setReportType] = useState<ReportType>("is");
  const [scope, setScope] = useState<ReportScope>("current_month");
  const [grouping, setGrouping] = useState<ReportGrouping>("");
  const [customBeginDate, setCustomBeginDate] = useState("");
  const [customEndDate, setCustomEndDate] = useState("");

  const reportQuery = useQuery({
    queryKey: ["report", reportType, scope, grouping, customBeginDate, customEndDate],
    queryFn: () => callCommand<ReportResult>("run_report", {
      reportType,
      interval: grouping,
      scope,
      beginDate: scope === "custom" ? customBeginDate || null : null,
      endDate: scope === "custom" ? customEndDate || null : null,
    }),
    enabled: false,
    retry: false,
  });

  const localizedReportTypes = useMemo(
    () => reportTypeOptions.map((opt) => ({ value: opt.value, label: t(opt.labelKey) })),
    [t],
  );

  const localizedScopes = useMemo(
    () => scopeOptions.map((opt) => ({ value: opt.value, label: t(opt.labelKey) })),
    [t],
  );

  const localizedGroupings = useMemo(
    () => groupingOptions.map((opt) => ({ value: opt.value, label: t(opt.labelKey) })),
    [t],
  );

  const columns = useMemo(() => {
    const cols: Parameters<typeof Table<ReportRow>>[0]["columns"] = [
      {
        title: t("transactions.account"),
        dataIndex: "account",
        render: (account: string, record: ReportRow) => (
          <span
            className={`${styles.account_cell}${record.isTotal ? ` ${styles.account_cell_total}` : ""}`}
            style={{ paddingLeft: `${(record.indent - 1) * 20}px` }}
          >
            {account}
          </span>
        ),
      },
    ];

    const periodColumns = reportQuery.data?.periodColumns ?? [];
    for (const period of periodColumns) {
      cols.push({
        title: period,
        align: "right" as const,
        width: 140,
        render: (_: unknown, record: ReportRow) => {
          const amt = record.amounts.find((a: ReportPeriodAmount) => a.period === period);
          if (!amt) return "-";
          return <Amount formatted={amt.formatted} tint={amt.tint} />;
        },
      });
    }

    cols.push({
      title: t("balances.value"),
      align: "right" as const,
      width: 140,
      render: (_: unknown, record: ReportRow) => (
        <Amount formatted={record.total.formatted} tint={record.total.tint} />
      ),
    });

    return cols;
  }, [reportQuery.data?.periodColumns, t]);

  const isLoading = reportQuery.isFetching;
  const canGenerate = scope !== "custom" || Boolean(customBeginDate && customEndDate);
  const data = reportQuery.data?.rows ?? [];
  const hasError = reportQuery.isError;
  const isEmpty =
    reportQuery.data &&
    data.length === 0 &&
    reportQuery.data.visualization.kind !== "cashflow" &&
    !isLoading;

  return (
    <div className={styles.container}>
      <Card size="small" className={styles.controls}>
        <Space wrap>
          <Select
            value={reportType}
            onChange={setReportType}
            options={localizedReportTypes}
            style={{ minWidth: 180 }}
          />
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
            onClick={() => reportQuery.refetch()}
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
        ) : hasError || isEmpty ? (
          <Empty
            className={styles.center_state}
            description={t("reports.generation_failed")}
          />
        ) : !reportQuery.data ? (
          <Empty
            className={styles.center_state}
            description={t("reports.select_and_generate")}
          />
        ) : (
          <>
            <ReportCharts data={reportQuery.data} />
            {showDetailedTable ? (
              <Collapse
                className={styles.detailed_table_collapse}
                defaultActiveKey={["detailed-table"]}
                items={[
                  {
                    key: "detailed-table",
                    label: t("reports.detailed_table"),
                    children: (
                      <Table<ReportRow>
                        rowKey={(row) => `${row.account}-${row.indent}-${String(row.isTotal)}`}
                        dataSource={data}
                        pagination={false}
                        scroll={{ x: "max-content" }}
                        columns={columns}
                      />
                    ),
                  },
                ]}
              />
            ) : null}
          </>
        )}
      </Card>
    </div>
  );
}
