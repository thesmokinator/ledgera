import { AutoComplete, Button, Form, Input } from "antd";
import { DeleteOutlined } from "@ant-design/icons";
import type { Rule } from "antd/es/form";
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

function singleLineRule(message: string): Rule {
  return {
    validator: (_: unknown, value: string) =>
      !value || !/[\r\n]/.test(value)
        ? Promise.resolve()
        : Promise.reject(new Error(message)),
  };
}

function numberRule(message: string): Rule {
  return {
    validator: (_: unknown, value: string) =>
      !value || /\d/.test(value)
        ? Promise.resolve()
        : Promise.reject(new Error(message)),
  };
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
