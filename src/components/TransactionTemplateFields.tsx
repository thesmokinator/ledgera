import {
  Alert,
  AutoComplete,
  DatePicker,
  Form,
  Input,
  Segmented,
  Select,
  Space,
} from "antd";
import type { FormInstance } from "antd";
import type { NamePath } from "antd/es/form/interface";
import dayjs from "dayjs";
import { useTranslation } from "react-i18next";
import type { TransactionType } from "../types";
import { isValidJournalDate, journalDateFormat } from "../utils/date";
import { AdvancedPostings, InvestmentFields, MovementFields, singleLineRule } from "./TransactionPostings";
import { usePostingRowLabels } from "./PostingRow";
import styles from "./TransactionModal.module.css";

export function transactionTemplateValidationFields({
  form,
  transactionType,
  isEditing,
  includeDate,
}: {
  form: FormInstance;
  transactionType: TransactionType;
  isEditing: boolean;
  includeDate: boolean;
}): NamePath[] {
  const baseFields: NamePath[] = includeDate ? ["date", "code", "description"] : ["code", "description"];

  if (transactionType === "movement" && !isEditing) {
    return [
      ...baseFields,
      ["postings", 0, "account"],
      ["postings", 0, "amount"],
      ["postings", 0, "commodity"],
      ["postings", 1, "comment"],
      ["postings", 1, "account"],
    ];
  }

  if (transactionType === "investment" && !isEditing) {
    return [
      ...baseFields,
      ["postings", 0, "account"],
      ["postings", 0, "commodity"],
      ["postings", 0, "amount"],
      ["postings", 0, "unitPrice"],
      ["postings", 0, "comment"],
      ["postings", 1, "account"],
      ["postings", 1, "commodity"],
      ["postings", 1, "amount"],
    ];
  }

  const postings = form.getFieldValue("postings") ?? [];
  return [
    ...baseFields,
    "postings",
    ...postings.flatMap((_: unknown, index: number): NamePath[] => [
      ["postings", index, "account"],
      ["postings", index, "commodity"],
      ["postings", index, "amount"],
      ["postings", index, "unitPrice"],
      ["postings", index, "comment"],
    ]),
  ];
}

export function TransactionTemplateFields({
  transactionType,
  isEditing,
  includeDate,
  includeModeField = false,
  showModeSelector,
  advancedEditNotice,
  codeOptions,
  descriptionOptions,
  accountOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
  descriptionPlaceholder,
  onTransactionTypeChange,
  validateFormSilently,
}: {
  transactionType: TransactionType;
  isEditing: boolean;
  includeDate: boolean;
  includeModeField?: boolean;
  showModeSelector: boolean;
  advancedEditNotice?: string;
  codeOptions: { value: string }[];
  descriptionOptions: { value: string }[];
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
  descriptionPlaceholder: string;
  onTransactionTypeChange?: (type: TransactionType) => void;
  validateFormSilently?: () => void;
}) {
  const { t } = useTranslation();
  const postingLabels = usePostingRowLabels(t);
  const isAdvancedMode = transactionType === "advanced" || isEditing;

  return (
    <>
      {includeModeField ? (
        <Form.Item name="mode" hidden>
          <Input />
        </Form.Item>
      ) : null}

      {showModeSelector ? (
        <Segmented<TransactionType>
          className={styles.transaction_type_selector}
          block
          value={transactionType}
          onChange={(value) => onTransactionTypeChange?.(value)}
          options={[
            { value: "movement", label: t("transactions.types.movement") },
            { value: "investment", label: t("transactions.types.investment") },
            { value: "advanced", label: t("transactions.types.advanced") },
          ]}
        />
      ) : advancedEditNotice ? (
        <Alert className={styles.form_error_alert} type="info" showIcon message={advancedEditNotice} />
      ) : null}

      <Space className={includeDate ? styles.form_row : styles.form_row_no_date} size="middle">
        {includeDate ? (
          <Form.Item
            label={t("transactions.date")}
            name="date"
            getValueProps={(value?: string) => ({
              value: value ? dayjs(value, journalDateFormat) : null,
            })}
            normalize={(value: dayjs.Dayjs | null) =>
              value ? value.format(journalDateFormat) : ""
            }
            rules={[
              { required: true, message: t("transactions.enter_transaction_date") },
              {
                validator: (_: unknown, value: string) =>
                  !value || isValidJournalDate(value)
                    ? Promise.resolve()
                    : Promise.reject(new Error(t("transactions.invalid_date"))),
              },
            ]}
          >
            <DatePicker format={journalDateFormat} className={styles.full_width_control} />
          </Form.Item>
        ) : null}
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
        <Form.Item
          label={t("transactions.code")}
          name="code"
          rules={[
            {
              validator: (_: unknown, value: string) =>
                !value || /^\(.+\)$/.test(value.trim())
                  ? Promise.resolve()
                  : Promise.reject(new Error(t("transactions.code_invalid"))),
            },
            singleLineRule(t("transactions.single_line_field")),
          ]}
        >
          <AutoComplete options={codeOptions} placeholder="(INV-001)" filterOption />
        </Form.Item>
      </Space>

      <Form.Item
        label={t("transactions.description")}
        name="description"
        rules={[singleLineRule(t("transactions.single_line_field"))]}
      >
        <AutoComplete options={descriptionOptions} placeholder={descriptionPlaceholder} filterOption />
      </Form.Item>

      {transactionType === "movement" && !isEditing ? (
        <MovementFields
          accountOptions={accountOptions}
          commodityOptions={commodityOptions}
          commentOptions={commentOptions}
          defaultCommodity={defaultCommodity}
        />
      ) : null}
      {transactionType === "investment" && !isEditing ? (
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
          validateFormSilently={validateFormSilently}
        />
      ) : null}
    </>
  );
}
