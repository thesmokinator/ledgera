import { AutoComplete, Button, Form, Input } from "antd";
import { DeleteOutlined } from "@ant-design/icons";
import { useMemo } from "react";
import { singleLineRule, numberRule } from "./TransactionPostings";
import styles from "./PostingRow.module.css";

export type PostingRowLabels = {
  account: string;
  commodity: string;
  amount: string;
  unitPrice: string;
  removeAriaLabel: string;
  commentPlaceholder: string;
  singleLineField: string;
  commodityRequired: string;
  amountInvalid: string;
  unitPriceInvalid: string;
  accountRequired: string;
};

export function usePostingRowLabels(t: (key: string) => string): PostingRowLabels {
  return useMemo(() => ({
    account: t("transactions.account"),
    commodity: t("transactions.commodity"),
    amount: t("transactions.amount"),
    unitPrice: t("transactions.unit_price"),
    removeAriaLabel: t("transactions.remove_posting"),
    commentPlaceholder: t("transactions.comment_placeholder"),
    singleLineField: t("transactions.single_line_field"),
    commodityRequired: t("transactions.commodity_required"),
    amountInvalid: t("transactions.amount_invalid"),
    unitPriceInvalid: t("transactions.unit_price_invalid"),
    accountRequired: t("transactions.account_required"),
  }), [t]);
}

export function PostingRow({
  field,
  accountOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
  labels,
  onRemove,
}: {
  field: { key: number; name: number };
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
  labels: PostingRowLabels;
  onRemove: () => void;
}) {
  return (
    <div className={styles.advanced_posting_row}>
      <Form.Item
        label={labels.account}
        name={[field.name, "account"]}
        rules={[
          { required: true, whitespace: true, message: labels.accountRequired },
          singleLineRule(labels.singleLineField),
        ]}
      >
        <AutoComplete options={accountOptions} placeholder="assets:bank" filterOption />
      </Form.Item>
      <Form.Item
        label={labels.commodity}
        name={[field.name, "commodity"]}
        rules={[singleLineRule(labels.singleLineField)]}
      >
        <AutoComplete options={commodityOptions} placeholder={defaultCommodity || "EUR"} filterOption />
      </Form.Item>
      <Form.Item
        label={labels.amount}
        name={[field.name, "amount"]}
        rules={[numberRule(labels.amountInvalid), singleLineRule(labels.singleLineField)]}
      >
        <Input placeholder="25.00" />
      </Form.Item>
      <Form.Item
        label={labels.unitPrice}
        name={[field.name, "unitPrice"]}
        rules={[numberRule(labels.unitPriceInvalid), singleLineRule(labels.singleLineField)]}
      >
        <Input placeholder={`150 ${defaultCommodity || "EUR"}`} />
      </Form.Item>
      <Button
        danger
        className={styles.posting_delete_button}
        aria-label={labels.removeAriaLabel}
        icon={<DeleteOutlined />}
        onClick={onRemove}
      />
      <Form.Item
        className={styles.posting_comment_field}
        name={[field.name, "comment"]}
        rules={[singleLineRule(labels.singleLineField)]}
      >
        <AutoComplete options={commentOptions} placeholder={labels.commentPlaceholder} filterOption />
      </Form.Item>
    </div>
  );
}
