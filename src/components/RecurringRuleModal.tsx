import {
  Alert,
  DatePicker,
  Form,
  Input,
  Modal,
  Select,
  Space,
} from "antd";
import dayjs from "dayjs";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { callCommand } from "../utils/command";
import { useRecurringRuleActions } from "../hooks/useRecurringRuleActions";
import type { PeriodicRule, PeriodicRuleInput, TransactionType } from "../types";
import { TransactionTemplateFields } from "./TransactionTemplateFields";
import styles from "./TransactionModal.module.css";

type RecurringFormValues = Omit<PeriodicRuleInput, 'startDate' | 'endDate'> & {
  _customPeriod?: string;
  startDate?: dayjs.Dayjs;
  endDate?: dayjs.Dayjs;
};

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

function emptyForm(defaultCommodity: string): RecurringFormValues {
  return {
    ruleId: "",
    periodExpr: "monthly",
    _customPeriod: "",
    description: "",
    postings: [
      { account: "", amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" },
      { account: "", amount: "", commodity: "", unitPrice: "", comment: "" },
    ],
    status: "",
    code: "",
    startDate: undefined,
    endDate: undefined,
  };
}

function ruleToForm(rule: PeriodicRule, defaultCommodity: string): RecurringFormValues {
  return {
    ruleId: rule.ruleId,
    periodExpr: BUILTIN_PERIODS.has(rule.periodExpr) ? rule.periodExpr : "custom",
    _customPeriod: BUILTIN_PERIODS.has(rule.periodExpr) ? "" : rule.periodExpr,
    description: rule.description,
    postings: rule.postings.map((p) => ({
      account: p.account,
      amount: p.amount,
      commodity: p.commodity || defaultCommodity,
      unitPrice: p.unitPrice,
      comment: p.comment,
    })),
    status: rule.status,
    code: rule.code,
    startDate: rule.startDate ? dayjs(rule.startDate) : undefined,
    endDate: rule.endDate ? dayjs(rule.endDate) : undefined,
  };
}

export function RecurringRuleModal({
  open,
  rule,
  accountOptions,
  codeOptions,
  descriptionOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
  onClose,
  onSaved,
}: {
  open: boolean;
  rule: PeriodicRule | null;
  accountOptions: { value: string }[];
  codeOptions: { value: string }[];
  descriptionOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useTranslation();
  const [form] = Form.useForm<RecurringFormValues>();
  const { submitRule, isSaving, saveError, clearSaveError } = useRecurringRuleActions({
    editingRule: rule,
    defaultCommodity,
    onSaved,
  });
  const [postingMode, setPostingMode] = useState<TransactionType>("movement");
  const [periodError, setPeriodError] = useState<string | null>(null);

  const isEditing = rule !== null;
  const title = isEditing ? t("recurring.edit_rule_title") : t("recurring.new_rule_title");
  const selectedPeriod = Form.useWatch("periodExpr", form);
  const showCustomPeriod = selectedPeriod === "custom";
  useEffect(() => {
    if (open) {
      form.resetFields();
      if (rule) {
        const initial = ruleToForm(rule, defaultCommodity);
        form.setFieldsValue(initial);
        setPostingMode("advanced");
      } else {
        form.setFieldsValue({ periodExpr: "monthly" });
        setPostingMode("movement");
      }
      clearSaveError();
      setPeriodError(null);
    }
  }, [open, rule, defaultCommodity, form, clearSaveError]);

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

  const handleFinish = (values: RecurringFormValues) => {
    submitRule(values);
  };

  const modalWidth = postingMode === "movement" && !isEditing ? 620 : 780;

  return (
    <Modal
      title={title}
      open={open}
      width={modalWidth}
      okText={isEditing ? t("common.save") : t("recurring.new_rule")}
      confirmLoading={isSaving}
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
        initialValues={emptyForm(defaultCommodity)}
      >
        <Form.Item
          name="ruleId"
          label={t("recurring.rule_name")}
          rules={[
            { required: true, whitespace: true, message: t("recurring.rule_name_required") },
            {
              pattern: /^\S+$/,
              message: t("recurring.rule_name_invalid"),
            },
          ]}
        >
          <Input placeholder={t("recurring.rule_name_placeholder")} autoFocus={!isEditing} />
        </Form.Item>

        <Space className={styles.schedule_row} size="middle">
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

        <TransactionTemplateFields
          transactionType={postingMode}
          isEditing={isEditing}
          includeDate={false}
          showModeSelector={!isEditing}
          advancedEditNotice={isEditing ? t("recurring.advanced_edit_notice") : undefined}
          codeOptions={codeOptions}
          descriptionOptions={descriptionOptions}
          accountOptions={accountOptions}
          commodityOptions={commodityOptions}
          commentOptions={commentOptions}
          defaultCommodity={defaultCommodity}
          descriptionPlaceholder={t("recurring.description_placeholder")}
          onTransactionTypeChange={setPostingMode}
        />
      </Form>
    </Modal>
  );
}
