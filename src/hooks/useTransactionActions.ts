import { useMutation, useQueryClient } from "@tanstack/react-query";
import dayjs from "dayjs";
import type { JournalSummary, JournalTransaction, TransactionInput } from "../types";
import { callCommand } from "../utils/command";
import { journalDateFormat } from "../utils/date";
import { parseError } from "../utils/error";
import { autoCalculateBalancingAmounts } from "../utils/transaction";

type Notifier = {
  success: (content: string) => void;
  error: (content: string) => void;
};

export function useTransactionActions({
  defaultCommodity,
  editingTransaction,
  messageApi,
  t,
  onSaved,
}: {
  defaultCommodity: string;
  editingTransaction: JournalTransaction | null;
  messageApi: Notifier;
  t: (key: string, options?: Record<string, unknown>) => string;
  onSaved: () => void;
}) {
  const queryClient = useQueryClient();

  async function invalidateJournalData() {
    await queryClient.invalidateQueries({ queryKey: ["transactions"] });
    await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
  }

  const createTransactionMutation = useMutation({
    mutationFn: (input: TransactionInput) =>
      callCommand<JournalSummary, { input: TransactionInput }>("create_transaction", { input }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transaction_created"));
      onSaved();
      await invalidateJournalData();
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  const updateTransactionMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: TransactionInput }) =>
      callCommand<JournalSummary, { id: string; input: TransactionInput }>("update_transaction", {
        id,
        input,
      }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transaction_updated"));
      onSaved();
      await invalidateJournalData();
    },
    onError: (error) => messageApi.error(parseError(error, t)),
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

  function submitTransaction(values: TransactionInput) {
    const rawPostings = (values.postings ?? [])
      .filter((posting) => posting.account.trim().length > 0)
      .map((posting) => ({
        account: posting.account,
        amount: posting.amount ?? "",
        commodity: posting.amount?.trim()
          ? (posting.commodity?.trim() || defaultCommodity)
          : (posting.commodity?.trim() || ""),
        unitPrice: posting.unitPrice ?? "",
        comment: posting.comment ?? "",
      }));

    const balancedPostings = autoCalculateBalancingAmounts(rawPostings, defaultCommodity);

    const normalizedValues: TransactionInput = {
      date: dayjs.isDayjs(values.date) ? values.date.format(journalDateFormat) : values.date,
      status: values.status ?? "",
      code: values.code ?? "",
      description: values.description,
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
    isSavingTransaction: createTransactionMutation.isPending || updateTransactionMutation.isPending,
    submitTransaction,
  };
}
