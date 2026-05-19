import { Card, Empty, Space, Table } from "antd";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Amount } from "../components/Amount";
import type { Balance, InvestmentOverview } from "./types";
import styles from "./BalancesRoute.module.css";

export function BalancesRoute({ fetchPrices }: { fetchPrices: boolean }) {
  const { t } = useTranslation();

  const balancesQuery = useQuery({
    queryKey: ["balances"],
    queryFn: () => invoke<Balance[]>("get_balances"),
    retry: false,
    refetchOnMount: true,
  });

  const investmentsQuery = useQuery({
    queryKey: ["investments-overview"],
    queryFn: () => invoke<InvestmentOverview[]>("get_investments_overview"),
    enabled: fetchPrices,
    retry: false,
    refetchOnMount: true,
  });

  const balances = balancesQuery.data ?? [];
  const investments = investmentsQuery.data ?? [];
  const isInitialLoadingBalances = balancesQuery.isLoading && !balancesQuery.data;
  const isInitialLoadingInvestments = investmentsQuery.isLoading && !investmentsQuery.data;

  const balanceColumns = [
    { title: t("transactions.account"), dataIndex: "account" },
    {
      title: t("transactions.amount"),
      align: "right" as const,
      width: 200,
      render: (_: unknown, record: Balance) => (
        <Amount
          formatted={record.formatted || "-"}
          tint={record.tint}
        />
      ),
    },
  ];

  const investmentColumns = [
    { title: t("balances.commodity"), dataIndex: "commodity", width: 140, render: (c: string) => <strong>{c}</strong> },
    { title: t("balances.account"), dataIndex: "account", ellipsis: true },
    {
      title: t("balances.quantity"),
      width: 140,
      align: "right" as const,
      render: (_: unknown, record: InvestmentOverview) => (
        <Amount
          formatted={record.quantityFormatted || record.quantity.toString()}
          tint={record.tint}
        />
      ),
    },
    ...(fetchPrices
      ? [
        {
          title: t("balances.price"), width: 140, align: "right" as const,
          render: (_: unknown, record: InvestmentOverview) => {
            if (!record.priceFormatted || !record.currency) return "-";
            return `${record.currency} ${record.priceFormatted}`;
          },
        },
        {
          title: t("balances.value"), width: 160, align: "right" as const,
          render: (_: unknown, record: InvestmentOverview) =>
            record.marketValueFormatted || "-",
        },
      ]
      : []),
  ];

  return (
    <Space direction="vertical" size={24} className="content-stack">
      {/* ── Account Balances ─────────────────────── */}
      <Card className={styles.card} title={t("balances.title")}>
        {isInitialLoadingBalances ? (
          <Table<Balance>
            rowKey="account"
            loading
            dataSource={[]}
            pagination={false}
            scroll={{ x: 400 }}
            columns={balanceColumns}
          />
        ) : balances.length === 0 ? (
          <Empty description={t("balances.empty")} />
        ) : (
          <Table<Balance>
            rowKey="account"
            loading={balancesQuery.isFetching}
            dataSource={balances}
            pagination={{ pageSize: 50 }}
            scroll={{ x: 400 }}
            columns={balanceColumns}
          />
        )}
      </Card>

      {/* ── Investments ──────────────────── */}
      {fetchPrices ? (
        <Card className={styles.card} title={t("balances.investments")}>
          {isInitialLoadingInvestments ? (
            <Table<InvestmentOverview>
              rowKey="commodity"
              loading
              dataSource={[]}
              pagination={false}
              scroll={{ x: 600 }}
              columns={investmentColumns}
            />
          ) : investments.length === 0 ? (
            <Empty description={t("balances.empty")} />
          ) : (
            <Table<InvestmentOverview>
              rowKey="commodity"
              loading={investmentsQuery.isFetching}
              dataSource={investments}
              pagination={false}
              scroll={{ x: 600 }}
              columns={investmentColumns}
            />
          )}
        </Card>
      ) : null}
    </Space>
  );
}
