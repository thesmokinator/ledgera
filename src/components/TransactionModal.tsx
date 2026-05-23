import {
  Alert,
  AutoComplete,
  Button,
  DatePicker,
  Form,
  Input,
  Modal,
  Segmented,
  Select,
  Space,
} from "antd";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import type { FormInstance } from "antd";
import type { NamePath } from "antd/es/form/interface";
import type { Rule } from "antd/es/form";
import dayjs from "dayjs";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { JournalTransaction, TransactionInput, TransactionType } from "../types";
import { isValidJournalDate, journalDateFormat } from "../utils/date";
import styles from "./TransactionModal.module.css";

function valueContainsCommodity(value: string): boolean {
  return /[^\d\s.,\-+]/.test(value);
}

function singleLineRule(message: string): Rule {
  return {
    validator: (_, value: string) =>
      !value || !/[\r\n]/.test(value)
        ? Promise.resolve()
        : Promise.reject(new Error(message)),
  };
}

function numberRule(message: string): Rule {
  return {
    validator: (_, value: string) =>
      !value || /\d/.test(value)
        ? Promise.resolve()
        : Promise.reject(new Error(message)),
  };
}

function requiredAmountRule(message: string): Rule {
  return {
    validator: (_, value: string) =>
      value?.trim() && /\d/.test(value)
        ? Promise.resolve()
        : Promise.reject(new Error(message)),
  };
}

function commodityRule({
  defaultCommodity,
  form,
  postingIndex,
  required,
  singleLineMessage,
  requiredMessage,
}: {
  defaultCommodity: string;
  form: FormInstance<TransactionInput>;
  postingIndex: number;
  required?: boolean;
  singleLineMessage: string;
  requiredMessage: string;
}): Rule {
  return {
    validator: (_, value: string) => {
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

function accountRules(t: (key: string) => string): Rule[] {
  return [
    { required: true, whitespace: true, message: t("transactions.account_required") },
    singleLineRule(t("transactions.single_line_field")),
  ];
}

function MovementFields({
  accountOptions,
  commodityOptions,
  defaultCommodity,
}: {
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  defaultCommodity: string;
}) {
  const { t } = useTranslation();
  const form = Form.useFormInstance<TransactionInput>();

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
      <Form.Item name={["postings", 0, "comment"]} rules={[singleLineRule(t("transactions.single_line_field"))]}>
        <Input placeholder={t("transactions.comment_placeholder")} />
      </Form.Item>
    </div>
  );
}

function InvestmentFields({
  accountOptions,
  commodityOptions,
  defaultCommodity,
}: {
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  defaultCommodity: string;
}) {
  const { t } = useTranslation();
  const form = Form.useFormInstance<TransactionInput>();

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
          <Input placeholder={`150 ${defaultCommodity || "EUR"}`} />
        </Form.Item>
        <Form.Item label={t("transactions.cash_account")} name={["postings", 1, "account"]} rules={accountRules(t)}>
          <AutoComplete options={accountOptions} placeholder="assets:bank" filterOption />
        </Form.Item>
        <Form.Item
          label={t("transactions.commodity")}
          name={["postings", 1, "commodity"]}
          rules={[singleLineRule(t("transactions.single_line_field"))]}
        >
          <AutoComplete options={commodityOptions} placeholder={defaultCommodity || "EUR"} filterOption />
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
        <Input placeholder={t("transactions.comment_placeholder")} />
      </Form.Item>
    </div>
  );
}

function AdvancedPostingRow({
  field,
  accountOptions,
  commodityOptions,
  defaultCommodity,
  onRemove,
}: {
  field: { key: number; name: number };
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  defaultCommodity: string;
  onRemove: () => void;
}) {
  const { t } = useTranslation();
  const form = Form.useFormInstance<TransactionInput>();

  return (
    <div className={styles.advanced_posting_row}>
      <Form.Item label={t("transactions.account")} name={[field.name, "account"]} rules={accountRules(t)}>
        <AutoComplete options={accountOptions} placeholder="assets:bank" filterOption />
      </Form.Item>
      <Form.Item
        label={t("transactions.commodity")}
        name={[field.name, "commodity"]}
        rules={[
          commodityRule({
            defaultCommodity,
            form,
            postingIndex: field.name,
            singleLineMessage: t("transactions.single_line_field"),
            requiredMessage: t("transactions.commodity_required"),
          }),
        ]}
      >
        <AutoComplete options={commodityOptions} placeholder={defaultCommodity || "EUR"} filterOption />
      </Form.Item>
      <Form.Item
        label={t("transactions.amount")}
        name={[field.name, "amount"]}
        rules={[numberRule(t("transactions.amount_invalid")), singleLineRule(t("transactions.single_line_field"))]}
      >
        <Input placeholder="25.00" />
      </Form.Item>
      <Form.Item
        label={t("transactions.unit_price")}
        name={[field.name, "unitPrice"]}
        rules={[numberRule(t("transactions.unit_price_invalid")), singleLineRule(t("transactions.single_line_field"))]}
      >
        <Input placeholder={`150 ${defaultCommodity || "EUR"}`} />
      </Form.Item>
      <Button
        danger
        className={styles.posting_delete_button}
        aria-label={t("transactions.remove_posting")}
        icon={<DeleteOutlined />}
        onClick={onRemove}
      />
      <Form.Item
        className={styles.posting_comment_field}
        name={[field.name, "comment"]}
        rules={[singleLineRule(t("transactions.single_line_field"))]}
      >
        <Input placeholder={t("transactions.comment_placeholder")} />
      </Form.Item>
    </div>
  );
}

function AdvancedPostings({
  accountOptions,
  commodityOptions,
  defaultCommodity,
  validateFormSilently,
}: {
  accountOptions: { value: string }[];
  commodityOptions: { value: string }[];
  defaultCommodity: string;
  validateFormSilently: () => void;
}) {
  const { t } = useTranslation();

  return (
    <Form.List
      name="postings"
      rules={[
        {
          validator: async (_, postings: TransactionInput["postings"] = []) => {
            const accountCount = postings.filter((posting) => posting?.account?.trim()).length;
            const amountCount = postings.filter((posting) => posting?.account?.trim() && posting?.amount?.trim()).length;

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
        <Space direction="vertical" className="content-stack">
          {fields.map((field) => (
            <AdvancedPostingRow
              key={field.key}
              field={field}
              accountOptions={accountOptions}
              commodityOptions={commodityOptions}
              defaultCommodity={defaultCommodity}
              onRemove={() => remove(field.name)}
            />
          ))}
          <Form.ErrorList errors={errors} />
          <Button
            icon={<PlusOutlined />}
            onClick={() => {
              add({ account: "", amount: "", commodity: defaultCommodity, unitPrice: "", comment: "" });
              window.setTimeout(validateFormSilently, 0);
            }}
          >
            {t("transactions.add_posting")}
          </Button>
        </Space>
      )}
    </Form.List>
  );
}

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
  defaultCommodity: string;
  saveError: string | null;
  onClose: () => void;
  onFormChange: () => void;
  onSubmit: (values: TransactionInput) => void;
  onTransactionTypeChange: (type: TransactionType) => void;
}) {
  const { t } = useTranslation();
  const [isFormValid, setFormValid] = useState(false);

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
                validator: (_, value: string) =>
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
                validator: (_, value: string) =>
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
            defaultCommodity={defaultCommodity}
          />
        ) : null}
        {transactionType === "investment" && !editingTransaction ? (
          <InvestmentFields
            accountOptions={accountOptions}
            commodityOptions={commodityOptions}
            defaultCommodity={defaultCommodity}
          />
        ) : null}
        {isAdvancedMode ? (
          <AdvancedPostings
            accountOptions={accountOptions}
            commodityOptions={commodityOptions}
            defaultCommodity={defaultCommodity}
            validateFormSilently={validateFormSilently}
          />
        ) : null}
      </Form>
    </Modal>
  );
}
