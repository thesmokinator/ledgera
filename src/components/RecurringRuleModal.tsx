import {
  Alert,
  AutoComplete,
  DatePicker,
  Form,
  Input,
  Modal,
  Segmented,
  Select,
  Space,
} from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { callCommand } from "../utils/command";
import type { PeriodicRule, PeriodicRuleInput, PeriodicRulesSummary } from "../types";
import { MovementFields, InvestmentFields, AdvancedPostings } from "./TransactionPostings";
import { usePostingRowLabels } from "./PostingRow";
import styles from "./RecurringRuleModal.module.css";

type RecurringFormValues = PeriodicRuleInput & { _customPeriod?: string };
type PostingMode = "movement" | "investment" | "advanced";

const PERIOD_OPTIONS = [
  { value: "daily", label: "recurring.period_daily" },
  { value: "weekly", label: "recurring.period_weekly" },
  { value: "biweekly", label: "recurring.period_biweekly" },
  { value: "monthly", label: "recurring.period_monthly" },
  { value: "bimonthly", label: "recurring.period_bimonthly" },
  { value: "quarterly", label: "recurring.period_quarterly" },
  { value: "yearly", label: "recurring.period_yearly" },
  { value: "custom", label: "recurring.period_custom" },
];

const BUILTIN_PERIODS = new Set(PERIOD_OPTIONS.filter((o) => o.value !== "custom").map((o) => o.value));

function emptyForm(): RecurringFormValues {
  return {
    ruleId: "",
    periodExpr: "monthly",
    _customPeriod: "",
    description: "",
    postings: [
      { account: "", amount: "", commodity: "", unitPrice: "", comment: "" },
      { account: "", amount: "", commodity: "", unitPrice: "", comment: "" },
    ],
    status: "",
    code: "",
    startDate: undefined,
    endDate: undefined,
  };
}

function ruleToForm(rule: PeriodicRule): RecurringFormValues {
  return {
    ruleId: rule.ruleId,
    periodExpr: BUILTIN_PERIODS.has(rule.periodExpr) ? rule.periodExpr : "custom",
    _customPeriod: BUILTIN_PERIODS.has(rule.periodExpr) ? "" : rule.periodExpr,
    description: rule.description,
    postings: rule.postings.map((p) => ({
      account: p.account,
      amount: p.amount,
      commodity: p.commodity,
      unitPrice: p.unitPrice,
      comment: p.comment,
    })),
    status: rule.status,
    code: rule.code,
    startDate: rule.startDate ?? undefined,
    endDate: rule.endDate ?? undefined,
  };
}

export function RecurringRuleModal({
  open,
  rule,
  accountOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
  onClose,
  onSaved,
}: {
  open: boolean;
  rule: PeriodicRule | null;
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation();
  const [form] = Form.useForm<RecurringFormValues>();
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [postingMode, setPostingMode] = useState<PostingMode>("movement");
  const [periodError, setPeriodError] = useState<string | null>(null);

  const isEditing = rule !== null;
  const title = isEditing ? t("recurring.edit_rule_title") : t("recurring.new_rule_title");
  const selectedPeriod = Form.useWatch("periodExpr", form);
  const showCustomPeriod = selectedPeriod === "custom";

  const postingLabels = usePostingRowLabels(t);

  useEffect(() => {
    if (open) {
      if (rule) {
        const initial = ruleToForm(rule);
        form.setFieldsValue(initial);
        setPostingMode("advanced");
      } else {
        form.resetFields();
        form.setFieldsValue({ periodExpr: "monthly" });
        setPostingMode("movement");
      }
      setSaveError(null);
      setPeriodError(null);
    }
  }, [open, rule, form]);

  const handleCustomPeriodBlur = useCallback(async () => {
    const expr = form.getFieldValue("_customPeriod")?.trim();
    if (!expr || BUILTIN_PERIODS.has(expr)) {
      setPeriodError(null);
      return;
    }
    setPeriodError(null);
    try {
      const valid = await callCommand<boolean>("validate_period_expression", { expr });
      if (!valid) {
        setPeriodError(t("recurring.period_invalid"));
      }
    } catch {
      setPeriodError(t("recurring.period_invalid"));
    }
  }, [form, t]);

  const handleFinish = useCallback(
    async (values: RecurringFormValues) => {
        const { _customPeriod, ...ruleValues } = values;
        const customPeriod = _customPeriod?.trim();
        const periodExpr = customPeriod || values.periodExpr;

      setSaving(true);
      setSaveError(null);
      try {
        const input: PeriodicRuleInput = {
          ...ruleValues,
          periodExpr,
        };
        if (isEditing && rule) {
          await callCommand<PeriodicRulesSummary>("update_periodic_rule", {
            ruleIdParam: rule.ruleId,
            input,
          });
        } else {
          await callCommand<PeriodicRulesSummary>("create_periodic_rule", { input });
        }
        onSaved();
      } catch (err) {
        setSaveError(String(err));
      } finally {
        setSaving(false);
      }
    },
    [isEditing, rule, onSaved],
  );

  const isAdvancedMode = postingMode === "advanced" || isEditing;
  const modalWidth = postingMode === "movement" && !isEditing ? 620 : 780;

  return (
    <Modal
      title={title}
      open={open}
      width={modalWidth}
      okText={isEditing ? t("common.save") : t("recurring.new_rule")}
      confirmLoading={saving}
      destroyOnHidden
      onCancel={onClose}
      onOk={() => form.submit()}
    >
      {saveError && (
        <Alert
          className={styles.form_error_alert}
          type="error"
          showIcon
          message={t("recurring.save_failed")}
          description={saveError}
        />
      )}
      <Form<RecurringFormValues>
        form={form}
        layout="vertical"
        onFinish={handleFinish}
        autoComplete="off"
        initialValues={emptyForm()}
      >
        <Form.Item
          name="ruleId"
          label={t("recurring.rule_name")}
          rules={[{ required: true, message: t("recurring.rule_name_required") }]}
        >
          <Input placeholder={t("recurring.rule_name_placeholder")} autoFocus={!isEditing} />
        </Form.Item>

        <Form.Item
          name="periodExpr"
          label={t("recurring.period_expr")}
          rules={[{ required: true, message: t("recurring.period_required") }]}
        >
          <Select
            options={PERIOD_OPTIONS.map((o) => ({
              value: o.value,
              label: t(o.label),
            }))}
          />
        </Form.Item>

        {showCustomPeriod && (
          <Form.Item name="_customPeriod" label={t("recurring.period_custom_label")}>
            <Input
              placeholder={t("recurring.custom_period_placeholder")}
              onBlur={handleCustomPeriodBlur}
              autoFocus
            />
          </Form.Item>
        )}
        {periodError && <Alert type="error" message={periodError} style={{ marginBottom: 12 }} />}

        <Space className={styles.date_row} size="middle">
          <Form.Item
            label={t("recurring.start_date")}
            name="startDate"
            rules={[{ required: true, message: t("recurring.start_date_required") }]}
          >
            <DatePicker className={styles.full_width_control} format="YYYY-MM-DD" placeholder="YYYY-MM-DD" />
          </Form.Item>
          <Form.Item name="endDate" label={t("recurring.end_date")}>
            <DatePicker className={styles.full_width_control} format="YYYY-MM-DD" placeholder="YYYY-MM-DD" />
          </Form.Item>
        </Space>

        <Space className={styles.form_row} size="middle">
          <Form.Item label={t("transactions.status")} name="status">
            <Select
              allowClear
              className={styles.full_width_control}
              placeholder={t("transactions.status_placeholder")}
              options={[
                { value: "*", label: t("transactions.status_cleared") },
                { value: "!", label: t("transactions.status_pending") },
              ]}
            />
          </Form.Item>
          <Form.Item label={t("transactions.code")} name="code">
            <AutoComplete className={styles.full_width_control} placeholder="(INV-001)" />
          </Form.Item>
        </Space>

        <Form.Item name="description" label={t("transactions.description")}>
          <AutoComplete options={[]} placeholder={t("recurring.description_placeholder")} />
        </Form.Item>

        {!isEditing && (
          <Segmented<PostingMode>
            className={styles.transaction_type_selector}
            block
            value={postingMode}
            onChange={setPostingMode}
            options={[
              { value: "movement", label: t("transactions.types.movement") },
              { value: "investment", label: t("transactions.types.investment") },
              { value: "advanced", label: t("transactions.types.advanced") },
            ]}
          />
        )}

        {postingMode === "movement" && !isEditing ? (
          <MovementFields
            accountOptions={accountOptions}
            commodityOptions={commodityOptions}
            commentOptions={commentOptions}
            defaultCommodity={defaultCommodity}
          />
        ) : null}
        {postingMode === "investment" && !isEditing ? (
          <InvestmentFields
            accountOptions={accountOptions}
            commodityOptions={commodityOptions}
            commentOptions={commentOptions}
            defaultCommodity={defaultCommodity}
          />
        ) : null}
        {isAdvancedMode ? (
          <AdvancedPostings
            accountOptions={accountOptions}
            commodityOptions={commodityOptions}
            commentOptions={commentOptions}
            defaultCommodity={defaultCommodity}
            labels={postingLabels}
          />
        ) : null}
      </Form>
    </Modal>
  );
}
