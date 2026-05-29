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
import type { FormInstance } from "antd";
import type { NamePath } from "antd/es/form/interface";
import dayjs from "dayjs";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { JournalTransaction, TransactionInput, TransactionType } from "../types";
import { isValidJournalDate, journalDateFormat } from "../utils/date";
import { MovementFields, InvestmentFields, AdvancedPostings, singleLineRule } from "./TransactionPostings";
import type { PostingRowLabels } from "./PostingRow";
import styles from "./TransactionModal.module.css";

export function TransactionModal({
  open,
  editingTransaction,
  transactionForm,
  isSaving,
  transactionType,
  codeOptions,
  descriptionOptions,
  accountOptions,
  commodityOptions,
  commentOptions,
  defaultCommodity,
  saveError,
  onClose,
  onFormChange,
  onSubmit,
  onTransactionTypeChange,
}: {
  open: boolean;
  editingTransaction: JournalTransaction | null;
  transactionForm: FormInstance<TransactionInput>;
  isSaving: boolean;
  transactionType: TransactionType;
  codeOptions: { value: string }[];
  descriptionOptions: { value: string }[];
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  commentOptions: { value: string }[];
  defaultCommodity: string;
  saveError: string | null;
  onClose: () => void;
  onFormChange: () => void;
  onSubmit: (values: TransactionInput) => void;
  onTransactionTypeChange: (type: TransactionType) => void;
}) {
  const { t } = useTranslation();
  const [isFormValid, setFormValid] = useState(false);

  const postingLabels = useMemo<PostingRowLabels>(() => ({
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

  function currentValidationFields(): NamePath[] {
    const baseFields: NamePath[] = ["date", "code", "description"];

    if (transactionType === "movement" && !editingTransaction) {
      return [
        ...baseFields,
        ["postings", 0, "account"],
        ["postings", 0, "amount"],
        ["postings", 0, "commodity"],
        ["postings", 0, "comment"],
        ["postings", 1, "account"],
      ];
    }

    if (transactionType === "investment" && !editingTransaction) {
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

    const postings = transactionForm.getFieldValue("postings") ?? [];
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

  function validateFormSilently() {
    transactionForm
      .validateFields(currentValidationFields(), { validateOnly: true })
      .then(() => setFormValid(true))
      .catch(() => setFormValid(false));
  }

  useEffect(() => {
    if (!open) {
      setFormValid(false);
      return;
    }

    const timer = window.setTimeout(validateFormSilently, 0);
    return () => window.clearTimeout(timer);
  }, [open, transactionForm, transactionType, defaultCommodity]);

  const isAdvancedMode = transactionType === "advanced" || Boolean(editingTransaction);
  const modalWidth = transactionType === "movement" && !editingTransaction ? 620 : 780;

  return (
    <Modal
      title={editingTransaction ? t("transactions.edit_transaction") : t("transactions.new_transaction")}
      open={open}
      width={modalWidth}
      okText={editingTransaction ? t("common.save") : t("transactions.create_transaction")}
      confirmLoading={isSaving}
      destroyOnHidden
      okButtonProps={{ disabled: !isFormValid || isSaving }}
      onCancel={onClose}
      onOk={() => transactionForm.submit()}
    >
      {saveError ? (
        <Alert
          className={styles.form_error_alert}
          type="error"
          showIcon
          message={t("transactions.save_failed")}
          description={saveError}
        />
      ) : null}
      <Form<TransactionInput>
        className={styles.transaction_form}
        form={transactionForm}
        layout="vertical"
        onValuesChange={() => {
          onFormChange();
          window.setTimeout(validateFormSilently, 0);
        }}
        onFinish={onSubmit}
      >
        {!editingTransaction ? (
          <>
            <Form.Item name="mode" hidden>
              <Input />
            </Form.Item>
            <Segmented<TransactionType>
              className={styles.transaction_type_selector}
              block
              value={transactionType}
              onChange={onTransactionTypeChange}
              options={[
                { value: "movement", label: t("transactions.types.movement") },
                { value: "investment", label: t("transactions.types.investment") },
                { value: "advanced", label: t("transactions.types.advanced") },
              ]}
            />
          </>
        ) : (
          <Alert className={styles.form_error_alert} type="info" showIcon message={t("transactions.advanced_edit_notice")} />
        )}

        <Space className={styles.form_row} size="middle">
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
          <AutoComplete options={descriptionOptions} placeholder={t("transactions.description_placeholder")} filterOption />
        </Form.Item>

        {transactionType === "movement" && !editingTransaction ? (
          <MovementFields
            accountOptions={accountOptions}
            commodityOptions={commodityOptions}
            commentOptions={commentOptions}
            defaultCommodity={defaultCommodity}
          />
        ) : null}
        {transactionType === "investment" && !editingTransaction ? (
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
      </Form>
    </Modal>
  );
}
