import { Form } from "antd";
import { useState } from "react";
import type {
  AppSettings,
  AutocompleteSuggestions,
  JournalTransaction,
  TransactionInput,
  TransactionType,
} from "../types";
import { transactionTemplatePostings } from "../utils/account";
import { todayJournalDate } from "../utils/date";
import {
  emptyTransaction,
  toTransactionInput,
  withDefaultCommodity,
} from "../utils/transaction";

export function useTransactionModal({
  activeSettings,
  autocompleteSuggestions,
  defaultCommodity,
}: {
  activeSettings: AppSettings;
  autocompleteSuggestions: AutocompleteSuggestions;
  defaultCommodity: string;
}) {
  const [transactionType, setTransactionType] = useState<TransactionType>("movement");
  const [editingTransaction, setEditingTransaction] = useState<JournalTransaction | null>(null);
  const [isTransactionModalOpen, setTransactionModalOpen] = useState(false);
  const [transactionForm] = Form.useForm<TransactionInput>();

  function applyTransactionType(type: TransactionType) {
    setTransactionType(type);
    transactionForm.setFieldValue("mode", type);
    transactionForm.setFieldValue(
      "postings",
      activeSettings.prefillPostings
        ? transactionTemplatePostings(type, autocompleteSuggestions, defaultCommodity)
        : withDefaultCommodity(emptyTransaction.postings, defaultCommodity),
    );
  }

  function openCreateTransaction() {
    setEditingTransaction(null);
    setTransactionType("movement");
    transactionForm.setFieldsValue({
      ...emptyTransaction,
      mode: "movement",
      date: todayJournalDate(),
      postings: activeSettings.prefillPostings
        ? transactionTemplatePostings("movement", autocompleteSuggestions, defaultCommodity)
        : withDefaultCommodity(emptyTransaction.postings, defaultCommodity),
    });
    setTransactionModalOpen(true);
  }

  function openEditTransaction(transaction: JournalTransaction) {
    setEditingTransaction(transaction);
    setTransactionType("advanced");
    transactionForm.setFieldsValue(toTransactionInput(transaction, defaultCommodity));
    setTransactionModalOpen(true);
  }

  function closeTransactionModal() {
    setTransactionModalOpen(false);
  }

  function clearEditingTransaction() {
    setEditingTransaction(null);
  }

  return {
    transactionForm,
    transactionType,
    editingTransaction,
    isTransactionModalOpen,
    applyTransactionType,
    openCreateTransaction,
    openEditTransaction,
    closeTransactionModal,
    clearEditingTransaction,
  };
}
