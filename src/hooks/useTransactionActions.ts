import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { FormInstance } from "antd";
import dayjs from "dayjs";
import { useState } from "react";
import type { JournalSummary, JournalTransaction, TransactionInput, TransactionType } from "../types";
import { callCommand } from "../utils/command";
import { journalDateFormat } from "../utils/date";
import { formatAppError, parseAppError, parseError } from "../utils/error";
import { autoCalculateBalancingAmounts } from "../utils/transaction";

type Notifier = {
  success: (content: string) => void;
  error: (content: string) => void;
};

export function useTransactionActions({
  defaultCommodity,
  editingTransaction,
  transactionForm,
  transactionType,
  messageApi,
  t,
  onSaved,
}: {
  defaultCommodity: string;
  editingTransaction: JournalTransaction | null;
  transactionForm: FormInstance<TransactionInput>;
  transactionType: TransactionType;
  messageApi: Notifier;
  t: (key: string, options?: Record<string, unknown>) => string;
  onSaved: () => void;
}) {
  const queryClient = useQueryClient();
  const [transactionError, setTransactionError] = useState<string | null>(null);

  function toNamePath(path: string[]): (string | number)[] {
    return path.map((part) => (/^\d+$/.test(part) ? Number(part) : part));
  }

  function handleSaveError(error: unknown) {
    const appError = parseAppError(error);
    const errorMessage = appError ? formatAppError(appError, t) : parseError(error, t);

    if (appError?.fieldErrors?.length) {
      const formFieldErrors = appError.fieldErrors.map((fieldError) => ({
        name: toNamePath(fieldError.path),
        errors: [fieldError.message],
      })) as Parameters<typeof transactionForm.setFields>[0];
      transactionForm.setFields(formFieldErrors);
    }

    setTransactionError(errorMessage);
    messageApi.error(errorMessage.split("\n")[0]);
  }

  async function invalidateJournalData() {
    await queryClient.invalidateQueries({ queryKey: ["transactions"] });
    await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
  }

  const createTransactionMutation = useMutation({
    mutationFn: (input: TransactionInput) =>
      callCommand<JournalSummary, { input: TransactionInput }>("create_transaction", { input }),
    onSuccess: async () => {
      setTransactionError(null);
      messageApi.success(t("transactions.transaction_created"));
      onSaved();
      await invalidateJournalData();
    },
    onError: handleSaveError,
  });

  const updateTransactionMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: TransactionInput }) =>
      callCommand<JournalSummary, { id: string; input: TransactionInput }>("update_transaction", {
        id,
        input,
      }),
    onSuccess: async () => {
      setTransactionError(null);
      messageApi.success(t("transactions.transaction_updated"));
      onSaved();
      await invalidateJournalData();
    },
    onError: handleSaveError,
  });

  const deleteTransactionMutation = useMutation({
    mutationFn: (id: string) =>
      callCommand<JournalSummary, { id: string }>("delete_transaction", { id }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transaction_deleted"));
      await invalidateJournalData();
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  function makeOutgoingAmount(value: string): string {
    const trimmed = value.trim();
    if (!trimmed || trimmed.startsWith("-")) return trimmed;
    return `-${trimmed}`;
  }

  function submitTransaction(values: TransactionInput) {
    setTransactionError(null);
    const rawPostings = (values.postings ?? []).map((posting) => ({
      account: posting.account ?? "",
      amount: posting.amount ?? "",
      commodity: posting.amount?.trim()
        ? (posting.commodity?.trim() || defaultCommodity)
        : (posting.commodity?.trim() || ""),
      unitPrice: posting.unitPrice ?? "",
      comment: posting.comment ?? "",
    }));

    const preparedPostings = transactionType === "movement" && !editingTransaction
      ? rawPostings.map((posting, index) =>
        index === 0
          ? { ...posting, amount: makeOutgoingAmount(posting.amount) }
          : { ...posting, amount: posting.amount ?? "" },
      )
      : rawPostings;

    const balancedPostings = autoCalculateBalancingAmounts(preparedPostings, defaultCommodity);

    const normalizedValues: TransactionInput = {
      mode: editingTransaction ? "advanced" : transactionType,
      date: dayjs.isDayjs(values.date) ? values.date.format(journalDateFormat) : values.date,
      status: values.status ?? "",
      code: values.code ?? "",
      description: values.description ?? "",
      postings: balancedPostings,
    };

    if (editingTransaction) {
      updateTransactionMutation.mutate({ id: editingTransaction.id, input: normalizedValues });
      return;
    }

    createTransactionMutation.mutate(normalizedValues);
  }

  return {
    deleteTransaction: (id: string) => deleteTransactionMutation.mutate(id),
    clearTransactionError: () => setTransactionError(null),
    isSavingTransaction: createTransactionMutation.isPending || updateTransactionMutation.isPending,
    submitTransaction,
    transactionError,
  };
}
