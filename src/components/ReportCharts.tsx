import { Column, Pie } from "@ant-design/charts";
import { Empty, theme } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ReportChartEntry, ReportPeriodSummary, ReportResult } from "../types";
import { chartColorPalette } from "../utils/reportCharts";
import styles from "./ReportCharts.module.css";

interface ReportChartsProps {
  data: ReportResult;
}

const maxPieSlices = 8;

function compactAxisValue(value: number): string {
  const absolute = Math.abs(value);
  if (absolute >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (absolute >= 1_000) return `${(value / 1_000).toFixed(0)}K`;
  return String(value);
}

function RankedBars({
  entries,
  colors,
}: {
  entries: ReportChartEntry[];
  colors: string[];
}) {
  const maxAmount = Math.max(...entries.map((entry) => entry.chartAmount), 1);

  return (
    <div className={styles.ranked_bars}>
      {entries.map((entry, index) => {
        const width = `${Math.max((entry.chartAmount / maxAmount) * 100, 2)}%`;
        const color = colors[index % colors.length];

        return (
          <div
            key={entry.account}
            className={styles.ranked_bar_row}
            title={`${entry.account}: ${entry.formatted}`}
          >
            <div className={styles.ranked_bar_header}>
              <span className={styles.ranked_bar_account}>{entry.account}</span>
              <span className={styles.ranked_bar_amount}>{entry.chartAmountFormatted}</span>
            </div>
            <div className={styles.ranked_bar_track}>
              <div
                className={styles.ranked_bar_fill}
                style={{ width, backgroundColor: color }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function ReportCharts({ data }: ReportChartsProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { visualization } = data;

  const colors = useMemo(
    () => chartColorPalette(Math.max(visualization.entries.length, visualization.periods.length)),
    [visualization.entries.length, visualization.periods.length],
  );

  const secondaryTextColor = token.colorTextSecondary;
  const gridColor = token.colorBorderSecondary;

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

  if (visualization.kind === "allocation") {
    const canUsePie = visualization.entries.length <= maxPieSlices;
    return (
      <div className={styles.single_chart_card}>
        <h4 className={styles.chart_title}>{t("reports.chart_allocation")}</h4>
        <div className={styles.chart_container}>
          {canUsePie ? (
            <Pie
              data={visualization.entries}
              angleField="chartAmount"
              colorField="account"
              color={colors}
              autoFit
              radius={0.8}
              innerRadius={0.5}
              label={false}
              legend={{
                color: {
                  position: "bottom",
                  layout: { justifyContent: "center" },
                  ...legendStyle,
                },
              }}
              tooltip={{
                title: "account",
                items: [
                  {
                    field: "formatted",
                    name: "",
                  },
                ],
              }}
            />
          ) : (
            <RankedBars entries={visualization.entries} colors={colors} />
          )}
        </div>
      </div>
    );
  }

  if (visualization.kind === "cashflow") {
    const hasPeriods = visualization.periods.length > 0;

    return (
      <div className={styles.single_chart_card}>
        <h4 className={styles.chart_title}>{t("reports.chart_cashflow")}</h4>
        <div className={styles.chart_container}>
          {hasPeriods ? (
            <Column
              data={visualization.periods}
              xField="period"
              yField="amount"
              color={colors[0]}
              autoFit
              legend={false}
              style={{
                maxWidth: 56,
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
                title: (d: ReportPeriodSummary) => d.period,
                items: [
                  {
                    field: "formatted",
                    name: "",
                  },
                ],
              }}
            />
          ) : (
            <Empty
              className={styles.chart_empty_state}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("reports.chart_cashflow_empty")}
            />
          )}
        </div>
      </div>
    );
  }

  return (
    <div className={styles.single_chart_card}>
      <h4 className={styles.chart_title}>{t("reports.chart_breakdown")}</h4>
      <div className={styles.chart_container}>
        <RankedBars entries={visualization.entries} colors={colors} />
      </div>
      <p className={styles.chart_note}>
        {t("reports.chart_grouped_by", { level: visualization.accountLevel })}
      </p>
    </div>
  );
}
