import { Button, Card, Space, Table, Typography } from "antd";
import { ReloadOutlined } from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { Balance, Holding, PriceInfo } from "./types";
import styles from "./BalancesRoute.module.css";

export function BalancesRoute({ fetchPrices }: { fetchPrices: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const balancesQuery = useQuery({
    queryKey: ["balances"],
    queryFn: () => invoke<Balance[]>("get_balances"),
    retry: false,
  });

  const holdingsQuery = useQuery({
    queryKey: ["holdings"],
    queryFn: () => invoke<Holding[]>("get_holdings"),
    retry: false,
  });

  const holdings = holdingsQuery.data ?? [];
  const symbols = holdings.map((h) => h.commodity);

  const pricesQuery = useQuery({
    queryKey: ["prices", symbols],
    queryFn: () => invoke<Record<string, PriceInfo>>("fetch_prices", { symbols }),
    enabled: fetchPrices && symbols.length > 0,
    retry: false,
    staleTime: 60_000,
  });

  const prices = pricesQuery.data ?? {};

  const balances = balancesQuery.data ?? [];

  function renderAmount(amount: number, formatted: string) {
    return (
      <span style={{ color: amount < 0 ? "#ef4444" : amount > 0 ? "#22c55e" : undefined, fontWeight: 500 }}>
        {formatted}
      </span>
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
            pagination={false}
            showHeader={false}
            columns={[
              { dataIndex: "account" },
              {
                align: "right",
                width: 200,
                render: (_: unknown, record: Balance) => renderAmount(record.amount, record.formatted),
              },
            ]}
          />
        )}
      </Card>

      {/* ── Investment Holdings ──────────────────── */}
      {holdings.length > 0 ? (
        <Card className={styles.card} title={t("balances.holdings")}>
          <Table<Holding>
            dataSource={holdings}
            rowKey="commodity"
            loading={holdingsQuery.isFetching}
            pagination={false}
            columns={[
              { title: t("balances.commodity"), dataIndex: "commodity", width: 140, render: (c: string) => <strong>{c}</strong> },
              { title: t("balances.account"), dataIndex: "account", ellipsis: true },
              {
                title: t("balances.quantity"), dataIndex: "quantity", width: 140, align: "right",
                render: (q: number) => Number.isInteger(q) ? q.toString() : q.toFixed(4).replace(/\.?0+$/, ""),
              },
              ...(fetchPrices
                ? [
                  {
                    title: t("balances.price"), width: 140, align: "right" as const,
                    render: (_: unknown, record: Holding) => {
                      const info = prices[record.commodity];
                      return info ? `${info.currency} ${info.price.toFixed(2)}` : "—";
                    },
                  },
                  {
                    title: t("balances.value"), width: 160, align: "right" as const,
                    render: (_: unknown, record: Holding) => {
                      const info = prices[record.commodity];
                      return info ? `${info.currency} ${(record.quantity * info.price).toFixed(2)}` : "—";
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
