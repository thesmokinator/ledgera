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

const maxPieLabels = 8;

function compactAxisValue(value: number): string {
  const absolute = Math.abs(value);
  if (absolute >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (absolute >= 1_000) return `${(value / 1_000).toFixed(0)}K`;
  return String(value);
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

  const textColor = token.colorText;
  const secondaryTextColor = token.colorTextSecondary;
  const gridColor = token.colorBorderSecondary;
  const pieLabelsEnabled = pieData.length <= maxPieLabels;

  const axisStyle = {
    labelFill: secondaryTextColor,
    labelFillOpacity: 1,
    lineStroke: gridColor,
    tickStroke: gridColor,
  };

  const legendStyle = {
    itemLabelFill: secondaryTextColor,
    itemLabelFillOpacity: 1,
  };

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
                  ...legendStyle,
                },
              }}
              axis={{
                x: {
                  title: false,
                  ...axisStyle,
                },
                y: {
                  title: false,
                  labelFormatter: compactAxisValue,
                  gridStroke: gridColor,
                  gridStrokeOpacity: 0.35,
                  ...axisStyle,
                },
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
              angleField="chartAmount"
              colorField="account"
              color={pieColors}
              autoFit
              radius={0.8}
              innerRadius={0.5}
              label={pieLabelsEnabled
                ? {
                  text: "account",
                  position: "outside",
                  style: {
                    fontSize: 11,
                    fill: textColor,
                    fillOpacity: 0.9,
                  },
                }
                : false}
              legend={{
                color: {
                  position: "bottom",
                  layout: { justifyContent: "center" },
                  ...legendStyle,
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
