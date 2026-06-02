import { DeleteOutlined, EditOutlined, PlusOutlined, ThunderboltOutlined } from "@ant-design/icons";
import { Button, Card, Empty, message, Modal, Space, Table, Tag, Typography } from "antd";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { callCommand } from "../utils/command";
import type { GenerateResult, PeriodicRule, PeriodicRulesSummary } from "./types";
import { RecurringRuleModal } from "../components/RecurringRuleModal";
import { ConfirmDeleteModal } from "../components/ConfirmDeleteModal";
import styles from "./RecurringRoute.module.css";

export function RecurringRoute({
  accountOptions,
  codeOptions,
  descriptionOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
}: {
  accountOptions: { value: string }[];
  codeOptions: { value: string }[];
  descriptionOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [modalOpen, setModalOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<PeriodicRule | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<PeriodicRule | null>(null);
  const [generating, setGenerating] = useState(false);
  const [generateConfirmOpen, setGenerateConfirmOpen] = useState(false);
  const [generateResult, setGenerateResult] = useState<GenerateResult | null>(null);

  const rulesQuery = useQuery({
    queryKey: ["periodic-rules"],
    queryFn: () => callCommand<PeriodicRulesSummary>("list_periodic_rules"),
    retry: false,
  });

  const rules = rulesQuery.data?.rules ?? [];

  const openNew = useCallback(() => {
    setEditingRule(null);
    setModalOpen(true);
  }, []);

  const openEdit = useCallback((rule: PeriodicRule) => {
    setEditingRule(rule);
    setModalOpen(true);
  }, []);

  const closeModal = useCallback(() => {
    setModalOpen(false);
    setEditingRule(null);
  }, []);

  const onSaved = useCallback(() => {
    closeModal();
    queryClient.invalidateQueries({ queryKey: ["periodic-rules"] });
  }, [closeModal, queryClient]);

  const deleteMutation = useMutation({
    mutationFn: (ruleId: string) => callCommand<PeriodicRulesSummary>("delete_periodic_rule", { ruleIdParam: ruleId }),
    onSuccess: () => {
      setDeleteTarget(null);
      queryClient.invalidateQueries({ queryKey: ["periodic-rules"] });
    },
    onError: (err) => {
      message.error(String(err));
    },
  });

  const handleGenerate = useCallback(async () => {
    setGenerating(true);
    setGenerateResult(null);
    try {
      const result = await callCommand<GenerateResult>("generate_recurring_transactions", { ruleIdFilter: null });
      setGenerateResult(result);
      setGenerateConfirmOpen(false);
      queryClient.invalidateQueries({ queryKey: ["periodic-rules"] });
      queryClient.invalidateQueries({ queryKey: ["transactions"] });
    } catch (err) {
      message.error(String(err));
    } finally {
      setGenerating(false);
    }
  }, [queryClient]);

  const isLoading = rulesQuery.isFetching && rules.length === 0;

  return (
    <div className={`${styles.content_stack} content-stack`}>
      <Card size="small" className={styles.toolbar_card}>
        <div className={styles.toolbar}>
          <div className={styles.toolbar_left}>
            <Button type="primary" icon={<PlusOutlined />} onClick={openNew}>
              {t("recurring.new_rule")}
            </Button>
            <Button
              icon={<ThunderboltOutlined />}
              onClick={() => setGenerateConfirmOpen(true)}
              loading={generating}
              disabled={rules.length === 0}
            >
              {generating ? t("recurring.generating") : t("recurring.generate_all")}
            </Button>
          </div>
          {generateResult && generateResult.generated > 0 && (
            <Typography.Text type="secondary">
              {generateResult.generated === 1
                ? t("recurring.generated_count", { count: generateResult.generated })
                : t("recurring.generated_count_plural", { count: generateResult.generated })}
            </Typography.Text>
          )}
        </div>
      </Card>

      <Card className={styles.rules_card}>
        {!isLoading && rules.length === 0 ? (
          <Empty
            className={styles.center_state}
            description={
              <>
                <p>{t("recurring.empty")}</p>
                <p>{t("recurring.empty_description")}</p>
              </>
            }
          />
        ) : (
          <Table<PeriodicRule>
            dataSource={rules}
            rowKey={(r) => r.id}
            loading={rulesQuery.isFetching}
            pagination={{ pageSize: 12 }}
            columns={[
              {
                title: t("recurring.rule_name"),
                dataIndex: "ruleId",
                key: "ruleId",
                render: (ruleId: string, rule) => (
                  <Space>
                    {rule.status && <Tag color={rule.status === "*" ? "green" : "orange"}>{rule.status}</Tag>}
                    <Space direction="vertical" size={0}>
                      <span>{ruleId}</span>
                      {rule.description && (
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          {rule.description}
                        </Typography.Text>
                      )}
                    </Space>
                  </Space>
                ),
              },
              {
                title: t("recurring.table_period"),
                dataIndex: "periodExpr",
                key: "periodExpr",
                width: 130,
                render: (expr: string) => (
                  <Tag className={styles.period_tag}>{expr}</Tag>
                ),
              },
              {
                title: t("recurring.start_date"),
                dataIndex: "startDate",
                key: "startDate",
                width: 120,
                render: (date: string | null) =>
                  date ? <Typography.Text type="secondary">{date}</Typography.Text> : <Typography.Text type="secondary">{t("recurring.not_set")}</Typography.Text>,
              },
              {
                title: t("recurring.end_date"),
                dataIndex: "endDate",
                key: "endDate",
                width: 120,
                render: (date: string | null) =>
                  date ? <Typography.Text type="secondary">{date}</Typography.Text> : <Typography.Text type="secondary">{t("recurring.not_set")}</Typography.Text>,
              },
              {
                title: "",
                key: "actions",
                width: 100,
                render: (_, rule) => (
                  <Space>
                    <Button
                      type="text"
                      size="small"
                      icon={<EditOutlined />}
                      onClick={() => openEdit(rule)}
                      aria-label={t("recurring.edit_rule")}
                    />
                    <Button
                      type="text"
                      size="small"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={() => setDeleteTarget(rule)}
                      aria-label={t("common.delete")}
                    />
                  </Space>
                ),
              },
            ]}
          />
        )}
      </Card>

      <ConfirmDeleteModal
        open={!!deleteTarget}
        title={t("recurring.delete_title")}
        message={deleteTarget ? t("recurring.delete_message", { description: deleteTarget.ruleId }) : ""}
        loading={deleteMutation.isPending}
        onConfirm={() => deleteTarget && deleteMutation.mutate(deleteTarget.ruleId)}
        onCancel={() => setDeleteTarget(null)}
      />

      <Modal
        open={generateConfirmOpen}
        title={t("recurring.generate_confirm_title")}
        okText={t("recurring.generate_confirm_ok")}
        cancelText={t("common.cancel")}
        onOk={handleGenerate}
        onCancel={() => setGenerateConfirmOpen(false)}
        okButtonProps={{ loading: generating }}
        cancelButtonProps={{ disabled: generating }}
        closable={!generating}
        keyboard={!generating}
        maskClosable={!generating}
      >
        <p>{t("recurring.generate_confirm_message")}</p>
      </Modal>

      <RecurringRuleModal
        open={modalOpen}
        rule={editingRule}
        accountOptions={accountOptions}
        codeOptions={codeOptions}
        descriptionOptions={descriptionOptions}
        commodityOptions={commodityOptions}
        commentOptions={commentOptions}
        defaultCommodity={defaultCommodity}
        onClose={closeModal}
        onSaved={onSaved}
      />
    </div>
  );
}
