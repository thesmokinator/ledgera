import { Column, Pie } from "@ant-design/charts";
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

  const columnData = useMemo(() => toColumnChartData(data), [data]);
  const pieData = useMemo(() => toPieChartData(data.rows), [data]);
  const pieColors = useMemo(
    () => chartColorPalette(pieData.length),
    [pieData.length],
  );

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
              color={chartColorPalette(
                new Set(columnData.map((d) => d.account)).size,
              )}
              autoFit
              legend={{
                color: {
                  position: "bottom",
                  layout: { justifyContent: "center" },
                },
              }}
              axis={{
                x: { title: false },
                y: { title: false },
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
                style: { fontSize: 11 },
              }}
              legend={{
                color: {
                  position: "bottom",
                  layout: { justifyContent: "center" },
                },
              }}
              tooltip={{
                items: [
                  {
                    channel: "angle",
                    valueFormatter: (value: number) =>
                      // Use Intl for compact number formatting
                      Math.abs(value) >= 1_000_000
                        ? `${(value / 1_000_000).toFixed(1)}M`
                        : Math.abs(value) >= 1_000
                          ? `${(value / 1_000).toFixed(1)}K`
                          : String(value),
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
