import {
  Alert,
  Form,
  Modal,
} from "antd";
import type { FormInstance } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { JournalTransaction, TransactionInput, TransactionType } from "../types";
import { TransactionTemplateFields, transactionTemplateValidationFields } from "./TransactionTemplateFields";
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

  function validateFormSilently() {
    transactionForm
      .validateFields(
        transactionTemplateValidationFields({
          form: transactionForm,
          transactionType,
          isEditing: Boolean(editingTransaction),
          includeDate: true,
        }),
        { validateOnly: true },
      )
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
        <TransactionTemplateFields
          transactionType={transactionType}
          isEditing={Boolean(editingTransaction)}
          includeDate
          includeModeField
          showModeSelector={!editingTransaction}
          advancedEditNotice={editingTransaction ? t("transactions.advanced_edit_notice") : undefined}
          codeOptions={codeOptions}
          descriptionOptions={descriptionOptions}
          accountOptions={accountOptions}
          commodityOptions={commodityOptions}
          commentOptions={commentOptions}
          defaultCommodity={defaultCommodity}
          descriptionPlaceholder={t("transactions.description_placeholder")}
          onTransactionTypeChange={onTransactionTypeChange}
          validateFormSilently={validateFormSilently}
        />
      </Form>
    </Modal>
  );
}
