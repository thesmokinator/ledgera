import { Column, Pie } from "@ant-design/charts";
import { theme } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ReportResult } from "../types";
import {
  toColumnChartData,
  toPieChartData,
  chartColorPalette,
} from "../utils/reportCharts";
import styles from "./ReportCharts.module.css";

interface ReportChartsProps {
  data: ReportResult;
}

export function ReportCharts({ data }: ReportChartsProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const columnData = useMemo(() => toColumnChartData(data), [data]);
  const pieData = useMemo(() => toPieChartData(data.rows), [data]);

  const uniqueAccounts = useMemo(
    () => [...new Set(columnData.map((d) => d.account))],
    [columnData],
  );
  const columnColors = useMemo(
    () => chartColorPalette(uniqueAccounts.length),
    [uniqueAccounts.length],
  );
  const pieColors = useMemo(
    () => chartColorPalette(pieData.length),
    [pieData.length],
  );

  // Label color that adapts to the current Ant Design theme
  const labelColor = token.colorText;
  const labelFontSize = 11;

  const hasColumnData = columnData.length > 0;
  const hasPieData = pieData.length > 0;

  if (!hasColumnData && !hasPieData) {
    return null;
  }

  return (
    <div className={styles.charts_grid}>
      {hasColumnData && (
        <div className={styles.chart_card}>
          <h4 className={styles.chart_title}>
            {t("reports.chart_by_period")}
          </h4>
          <div className={styles.chart_container}>
            <Column
              data={columnData}
              xField="period"
              yField="amount"
              seriesField="account"
              color={columnColors}
              autoFit
              legend={{
                color: {
                  position: "bottom",
                  layout: { justifyContent: "center" },
                },
              }}
              axis={{
                x: { title: false },
                y: { title: false, labelFormatter: (value: number) => String(value) },
              }}
              tooltip={{
                title: (d: { period: string }) => d.period,
                items: [
                  {
                    name: (d: { account: string }) => d.account,
                    value: (d: { formatted: string }) => d.formatted,
                  },
                ],
              }}
            />
          </div>
        </div>
      )}

      {hasPieData && (
        <div className={styles.chart_card}>
          <h4 className={styles.chart_title}>
            {t("reports.chart_distribution")}
          </h4>
          <div className={styles.chart_container}>
            <Pie
              data={pieData}
              angleField="amount"
              colorField="account"
              color={pieColors}
              autoFit
              radius={0.8}
              innerRadius={0.5}
              label={{
                text: "account",
                position: "outside",
                style: {
                  fontSize: labelFontSize,
                  fill: labelColor,
                },
              }}
              legend={{
                color: {
                  position: "bottom",
                  layout: { justifyContent: "center" },
                  itemLabelFill: labelColor,
                },
              }}
              tooltip={{
                title: (d: { account: string }) => d.account,
                items: [
                  {
                    name: t("balances.value"),
                    value: (d: { formatted: string }) => d.formatted,
                  },
                ],
              }}
            />
          </div>
        </div>
      )}

      {data.rows.length > 0 && (
        <p className={styles.chart_note}>
          {t("reports.chart_table_below")}
        </p>
      )}
    </div>
  );
}
