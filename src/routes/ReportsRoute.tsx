import { BarChartOutlined } from "@ant-design/icons";
import { Button, Card, Collapse, Empty, Select, Space, Spin, Table } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { Amount } from "../components/Amount";
import { ReportCharts } from "../components/ReportCharts";
import type { ReportPeriodAmount, ReportResult, ReportRow } from "./types";
import styles from "./ReportsRoute.module.css";

type ReportType = "is" | "bs" | "cf";
type ReportInterval = "" | "-M" | "-Q" | "-Y";

const reportTypeOptions = [
  { value: "is" as const, labelKey: "reports.income_statement" },
  { value: "bs" as const, labelKey: "reports.balance_sheet" },
  { value: "cf" as const, labelKey: "reports.cashflow" },
];

const intervalOptions = [
  { value: "" as const, labelKey: "reports.no_interval" },
  { value: "-M" as const, labelKey: "reports.monthly" },
  { value: "-Q" as const, labelKey: "reports.quarterly" },
  { value: "-Y" as const, labelKey: "reports.yearly" },
];

export function ReportsRoute({
  showDetailedTable = false,
}: {
  showDetailedTable?: boolean;
}) {
  const { t } = useTranslation();
  const [reportType, setReportType] = useState<ReportType>("is");
  const [interval, setInterval] = useState<ReportInterval>("-M");

  const reportQuery = useQuery({
    queryKey: ["report", reportType, interval],
    queryFn: () => invoke<ReportResult>("run_report", { reportType, interval }),
    enabled: false,
    retry: false,
  });

  const localizedReportTypes = useMemo(
    () => reportTypeOptions.map((opt) => ({ value: opt.value, label: t(opt.labelKey) })),
    [t],
  );

  const localizedIntervals = useMemo(
    () => intervalOptions.map((opt) => ({ value: opt.value, label: t(opt.labelKey) })),
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
            value={interval}
            onChange={setInterval}
            options={localizedIntervals}
            style={{ minWidth: 140 }}
          />
          <Button
            type="primary"
            icon={<BarChartOutlined />}
            loading={isLoading}
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
