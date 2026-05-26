import { Bar, Column, Pie } from "@ant-design/charts";
import { theme } from "antd";
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

export function ReportCharts({ data }: ReportChartsProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { visualization } = data;

  const colors = useMemo(
    () => chartColorPalette(Math.max(visualization.entries.length, visualization.periods.length)),
    [visualization.entries.length, visualization.periods.length],
  );

  const textColor = token.colorText;
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
              label={{
                text: "label",
                position: "outside",
                style: {
                  fontSize: 11,
                  fill: textColor,
                  fillOpacity: 0.9,
                },
              }}
              legend={{
                color: {
                  position: "bottom",
                  layout: { justifyContent: "center" },
                  ...legendStyle,
                },
              }}
              tooltip={{
                title: (d: ReportChartEntry) => d.account,
                items: [
                  {
                    name: t("balances.value"),
                    value: (d: ReportChartEntry) => d.formatted,
                  },
                ],
              }}
            />
          ) : (
            <Bar
              data={visualization.entries}
              xField="chartAmount"
              yField="account"
              colorField="account"
              color={colors}
              autoFit
              legend={false}
              axis={{
                x: {
                  title: false,
                  labelFormatter: compactAxisValue,
                  gridStroke: gridColor,
                  gridStrokeOpacity: 0.35,
                  ...axisStyle,
                },
                y: {
                  title: false,
                  ...axisStyle,
                },
              }}
              tooltip={{
                title: (d: ReportChartEntry) => d.account,
                items: [
                  {
                    name: t("balances.value"),
                    value: (d: ReportChartEntry) => d.formatted,
                  },
                ],
              }}
            />
          )}
        </div>
      </div>
    );
  }

  if (visualization.kind === "cashflow") {
    return (
      <div className={styles.single_chart_card}>
        <h4 className={styles.chart_title}>{t("reports.chart_cashflow")}</h4>
        <div className={styles.chart_container}>
          <Column
            data={visualization.periods}
            xField="period"
            yField="amount"
            color={colors[0]}
            autoFit
            legend={false}
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
                  name: t("balances.value"),
                  value: (d: ReportPeriodSummary) => d.formatted,
                },
              ],
            }}
          />
        </div>
      </div>
    );
  }

  return (
    <div className={styles.single_chart_card}>
      <h4 className={styles.chart_title}>{t("reports.chart_breakdown")}</h4>
      <div className={styles.chart_container}>
        <Bar
          data={visualization.entries}
          xField="chartAmount"
          yField="account"
          colorField="account"
          color={colors}
          autoFit
          legend={false}
          axis={{
            x: {
              title: false,
              labelFormatter: compactAxisValue,
              gridStroke: gridColor,
              gridStrokeOpacity: 0.35,
              ...axisStyle,
            },
            y: {
              title: false,
              ...axisStyle,
            },
          }}
          tooltip={{
            title: (d: ReportChartEntry) => d.account,
            items: [
              {
                name: t("balances.value"),
                value: (d: ReportChartEntry) => d.formatted,
              },
            ],
          }}
        />
      </div>
      <p className={styles.chart_note}>
        {t("reports.chart_grouped_by", { level: visualization.accountLevel })}
      </p>
    </div>
  );
}
