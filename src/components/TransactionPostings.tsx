import { AutoComplete, Button, Form, Input, Space } from "antd";
import { PlusOutlined } from "@ant-design/icons";
import type { FormInstance } from "antd";
import type { Rule } from "antd/es/form";
import { useTranslation } from "react-i18next";
import { PostingRow, type PostingRowLabels } from "./PostingRow";
import styles from "./TransactionModal.module.css";

function valueContainsCommodity(value: string): boolean {
  return /[^\d\s.,\-+]/.test(value);
}

export function singleLineRule(message: string): Rule {
  return {
    validator: (_: unknown, value: string) =>
      !value || !/[\r\n]/.test(value)
        ? Promise.resolve()
        : Promise.reject(new Error(message)),
  };
}

export function numberRule(message: string): Rule {
  return {
    validator: (_: unknown, value: string) =>
      !value || /\d/.test(value)
        ? Promise.resolve()
        : Promise.reject(new Error(message)),
  };
}

export function requiredAmountRule(message: string): Rule {
  return {
    validator: (_: unknown, value: string) =>
      value?.trim() && /\d/.test(value)
        ? Promise.resolve()
        : Promise.reject(new Error(message)),
  };
}

export function commodityRule({
  defaultCommodity,
  form,
  postingIndex,
  required,
  singleLineMessage,
  requiredMessage,
}: {
  defaultCommodity: string;
  form: FormInstance;
  postingIndex: number;
  required?: boolean;
  singleLineMessage: string;
  requiredMessage: string;
}): Rule {
  return {
    validator: (_: unknown, value: string) => {
      const amount = String(form.getFieldValue(["postings", postingIndex, "amount"]) ?? "");
      if (value && /[\r\n]/.test(value)) {
        return Promise.reject(new Error(singleLineMessage));
      }
      if (required && !value?.trim()) {
        return Promise.reject(new Error(requiredMessage));
      }
      if (!defaultCommodity.trim() && amount.trim() && !value?.trim() && !valueContainsCommodity(amount)) {
        return Promise.reject(new Error(requiredMessage));
      }
      return Promise.resolve();
    },
  };
}

export function accountRules(t: (key: string) => string): Rule[] {
  return [
    { required: true, whitespace: true, message: t("transactions.account_required") },
    singleLineRule(t("transactions.single_line_field")),
  ];
}

export function MovementFields({
  accountOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
}: {
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
}) {
  const { t } = useTranslation();
  const form = Form.useFormInstance();

  return (
    <div className={styles.mode_stack}>
      <p className={styles.mode_hint}>{t("transactions.movement_hint")}</p>
      <div className={styles.movement_grid}>
        <Form.Item label={t("transactions.from_account")} name={["postings", 0, "account"]} rules={accountRules(t)}>
          <AutoComplete options={accountOptions} placeholder="assets:bank" filterOption />
        </Form.Item>
        <Form.Item label={t("transactions.to_account")} name={["postings", 1, "account"]} rules={accountRules(t)}>
          <AutoComplete options={accountOptions} placeholder="expenses:food" filterOption />
        </Form.Item>
        <Form.Item
          label={t("transactions.amount")}
          name={["postings", 0, "amount"]}
          rules={[requiredAmountRule(t("transactions.amount_required")), singleLineRule(t("transactions.single_line_field"))]}
        >
          <Input placeholder="25.00" />
        </Form.Item>
        <Form.Item
          label={t("transactions.commodity")}
          name={["postings", 0, "commodity"]}
          rules={[
            commodityRule({
              defaultCommodity,
              form,
              postingIndex: 0,
              singleLineMessage: t("transactions.single_line_field"),
              requiredMessage: t("transactions.commodity_required"),
            }),
          ]}
        >
          <AutoComplete options={commodityOptions} placeholder={defaultCommodity || "EUR"} filterOption />
        </Form.Item>
      </div>
      <Form.Item name={["postings", 1, "comment"]} rules={[singleLineRule(t("transactions.single_line_field"))]}>
        <AutoComplete options={commentOptions} placeholder={t("transactions.comment_placeholder")} filterOption />
      </Form.Item>
    </div>
  );
}

export function InvestmentFields({
  accountOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
}: {
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
}) {
  const { t } = useTranslation();
  const form = Form.useFormInstance();

  return (
    <div className={styles.mode_stack}>
      <p className={styles.mode_hint}>{t("transactions.investment_hint")}</p>
      <div className={styles.investment_grid}>
        <Form.Item label={t("transactions.investment_account")} name={["postings", 0, "account"]} rules={accountRules(t)}>
          <AutoComplete options={accountOptions} placeholder="assets:broker:VWCE" filterOption />
        </Form.Item>
        <Form.Item
          label={t("transactions.commodity")}
          name={["postings", 0, "commodity"]}
          rules={[
            commodityRule({
              defaultCommodity,
              form,
              postingIndex: 0,
              required: true,
              singleLineMessage: t("transactions.single_line_field"),
              requiredMessage: t("transactions.commodity_required"),
            }),
          ]}
        >
          <AutoComplete options={commodityOptions} placeholder="VWCE" filterOption />
        </Form.Item>
        <Form.Item
          label={t("transactions.quantity")}
          name={["postings", 0, "amount"]}
          rules={[requiredAmountRule(t("transactions.quantity_required")), singleLineRule(t("transactions.single_line_field"))]}
        >
          <Input placeholder="10" />
        </Form.Item>
        <Form.Item
          label={t("transactions.unit_price")}
          name={["postings", 0, "unitPrice"]}
          rules={[requiredAmountRule(t("transactions.unit_price_required")), singleLineRule(t("transactions.single_line_field"))]}
        >
          <Input placeholder={`150 $`} />
        </Form.Item>
        <Form.Item label={t("transactions.cash_account")} name={["postings", 1, "account"]} rules={accountRules(t)}>
          <AutoComplete options={accountOptions} placeholder="assets:bank" filterOption />
        </Form.Item>
        <Form.Item
          label={t("transactions.commodity")}
          name={["postings", 1, "commodity"]}
          rules={[singleLineRule(t("transactions.single_line_field"))]}
        >
          <AutoComplete options={commodityOptions} placeholder="EUR" filterOption />
        </Form.Item>
        <Form.Item
          label={t("transactions.cash_amount")}
          name={["postings", 1, "amount"]}
          rules={[numberRule(t("transactions.amount_invalid")), singleLineRule(t("transactions.single_line_field"))]}
        >
          <Input placeholder={t("transactions.auto_calculated_placeholder")} />
        </Form.Item>
      </div>
      <Form.Item name={["postings", 0, "comment"]} rules={[singleLineRule(t("transactions.single_line_field"))]}>
        <AutoComplete options={commentOptions} placeholder={t("transactions.comment_placeholder")} filterOption />
      </Form.Item>
    </div>
  );
}

export function AdvancedPostings({
  accountOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
  labels,
  validateFormSilently,
}: {
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
  labels: PostingRowLabels;
  validateFormSilently?: () => void;
}) {
  const { t } = useTranslation();

  return (
    <Form.List
      name="postings"
      rules={[
        {
          validator: async (_: unknown, postings: { account?: string; amount?: string }[] = []) => {
            const accountCount = postings.filter((p) => p?.account?.trim()).length;
            const amountCount = postings.filter((p) => p?.account?.trim() && p?.amount?.trim()).length;
            if (accountCount < 2) {
              throw new Error(t("transactions.postings_minimum"));
            }
            if (amountCount === 0) {
              throw new Error(t("transactions.postings_amount_required"));
            }
          },
        },
      ]}
    >
      {(fields, { add, remove }, { errors }) => (
        <Space orientation="vertical" className="content-stack">
          {fields.map((field) => (
            <PostingRow
              key={field.key}
              field={field}
              accountOptions={accountOptions}
              commodityOptions={commodityOptions}
              commentOptions={commentOptions}
              defaultCommodity={defaultCommodity}
              labels={labels}
              onRemove={() => remove(field.name)}
            />
          ))}
          <Form.ErrorList errors={errors} />
          <Button
            icon={<PlusOutlined />}
            onClick={() => {
              add({ account: "", amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" });
              window.setTimeout(() => validateFormSilently?.(), 0);
            }}
          >
            {t("transactions.add_posting")}
          </Button>
        </Space>
      )}
    </Form.List>
  );
}
