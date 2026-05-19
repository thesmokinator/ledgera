import { Button, Card, Space, Table, Typography } from "antd";
import { ReloadOutlined } from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Amount } from "../components/Amount";
import type { Balance, PriceInfo } from "./types";
import styles from "./BalancesRoute.module.css";

export function BalancesRoute({ fetchPrices }: { fetchPrices: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const balancesQuery = useQuery({
    queryKey: ["balances"],
    queryFn: () => invoke<Balance[]>("get_balances"),
    retry: false,
  });

  const investmentsQuery = useQuery({
    queryKey: ["investments"],
    queryFn: () => invoke<Balance[]>("get_investments"),
    retry: false,
  });

  const investments = investmentsQuery.data ?? [];
  const symbols = investments.map((h) => h.commodity);

  const pricesQuery = useQuery({
    queryKey: ["prices", symbols],
    queryFn: () => invoke<Record<string, PriceInfo>>("fetch_prices", { symbols }),
    enabled: fetchPrices && symbols.length > 0,
    retry: false,
    staleTime: 60_000,
  });

  const prices = pricesQuery.data ?? {};

  const marketValuesQuery = useQuery({
    queryKey: ["marketValues", investments, prices],
    queryFn: async () => {
      const result: Record<string, string> = {};
      for (const inv of investments) {
        const info = prices[inv.commodity];
        if (info) {
          const value = inv.amount * info.price;
          result[inv.commodity] = await invoke<string>("format_number", { value });
        }
      }
      return result;
    },
    enabled: investments.length > 0 && Object.keys(prices).length > 0,
    staleTime: 60_000,
  });

  const marketValues = marketValuesQuery.data ?? {};

  const balances = balancesQuery.data ?? [];

  function renderAmount(balance: Balance) {
    return (
      <Amount
        formatted={balance.formatted || "-"}
        tint={balance.tint}
      />
    );
  }

  return (
    <Space direction="vertical" size={24} className="content-stack">
      {/* ── Account Balances ─────────────────────── */}
      <Card
        className={styles.card}
        title={t("balances.title")}
        extra={
          fetchPrices ? (
            <Button
              icon={<ReloadOutlined />}
              loading={pricesQuery.isFetching}
              onClick={() => queryClient.invalidateQueries({ queryKey: ["prices"] })}
            >
              {t("common.refresh")}
            </Button>
          ) : null
        }
      >
        {balances.length === 0 ? (
          <Typography.Text type="secondary">{t("balances.empty")}</Typography.Text>
        ) : (
          <Table<Balance>
            dataSource={balances}
            rowKey="account"
            loading={balancesQuery.isFetching}
            pagination={{ pageSize: 50 }}
            scroll={{ x: 400 }}
            showHeader={false}
            columns={[
              { dataIndex: "account" },
              {
                align: "right",
                width: 200,
                render: (_: unknown, record: Balance) => renderAmount(record),
              },
            ]}
          />
        )}
      </Card>

      {/* ── Investments ──────────────────── */}
      {investments.length > 0 ? (
        <Card className={styles.card} title={t("balances.investments")}>
          <Table<Balance>
            dataSource={investments}
            rowKey="commodity"
            loading={investmentsQuery.isFetching}
            pagination={false}
            scroll={{ x: 600 }}
            columns={[
              { title: t("balances.commodity"), dataIndex: "commodity", width: 140, render: (c: string) => <strong>{c}</strong> },
              { title: t("balances.account"), dataIndex: "account", ellipsis: true },
              {
                title: t("balances.quantity"), width: 140, align: "right",
                render: (_: unknown, record: Balance) => (
                  <Amount
                    formatted={record.formatted || record.amount.toString()}
                    tint={record.tint}
                  />
                ),
              },
              ...(fetchPrices
                ? [
                  {
                    title: t("balances.price"), width: 140, align: "right" as const,
                    render: (_: unknown, record: Balance) => {
                      const info = prices[record.commodity];
                      return info ? `${info.currency} ${info.formatted}` : "-";
                    },
                  },
                  {
                    title: t("balances.value"), width: 160, align: "right" as const,
                    render: (_: unknown, record: Balance) => {
                      const info = prices[record.commodity];
                      if (!info) return "-";
                      const mv = marketValues[record.commodity];
                      return mv ? `${info.currency} ${mv}` : "-";
                    },
                  },
                ]
                : []),
            ]}
          />
        </Card>
      ) : null}
    </Space>
  );
}
