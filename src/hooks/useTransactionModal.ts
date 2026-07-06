import { Form } from "antd";
import { useRef, useState } from "react";
import type {
  AppSettings,
  AutocompleteSuggestions,
  JournalTransaction,
  PostingInput,
  TransactionInput,
  TransactionType,
} from "../types";
import { transactionTemplatePostings } from "../utils/account";
import { todayJournalDate } from "../utils/date";
import {
  emptyTransaction,
  makeMovementInputAmount,
  makeOutgoingAmount,
  toTransactionInput,
  withDefaultCommodity,
} from "../utils/transaction";

type SimpleTransactionType = Exclude<TransactionType, "advanced">;

function normalizePosting(
  posting: Partial<PostingInput> | undefined,
  fallback?: PostingInput,
): PostingInput {
  return {
    account: posting?.account ?? fallback?.account ?? "",
    amount: posting?.amount ?? fallback?.amount ?? "",
    commodity: posting?.commodity ?? fallback?.commodity ?? "",
    unitPrice: posting?.unitPrice ?? fallback?.unitPrice ?? "",
    comment: posting?.comment ?? fallback?.comment ?? "",
  };
}

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
  const advancedPostingsDraft = useRef<PostingInput[] | null>(null);

  function templatePostings(type: TransactionType): PostingInput[] {
    return activeSettings.prefillPostings
      ? transactionTemplatePostings(type, autocompleteSuggestions, defaultCommodity)
      : withDefaultCommodity(emptyTransaction.postings, defaultCommodity);
  }

  function currentPostingsWithFallback(type: TransactionType): PostingInput[] {
    const currentPostings = (transactionForm.getFieldValue("postings") ?? []) as Partial<PostingInput>[];
    const fallbackPostings = templatePostings(type);
    const sourcePostings = currentPostings.length > 0 ? currentPostings : fallbackPostings;
    const postings = sourcePostings.map((posting, index) =>
      normalizePosting(posting, fallbackPostings[index]),
    );

    while (postings.length < 2) {
      postings.push(normalizePosting(undefined, fallbackPostings[postings.length]));
    }

    return postings;
  }

  function currentPostingsForAdvancedMode(): PostingInput[] {
    const postings = currentPostingsWithFallback("advanced");

    if (transactionType === "movement" && postings[0]?.amount) {
      postings[0] = {
        ...postings[0],
        amount: makeOutgoingAmount(postings[0].amount),
      };
    }

    const savedExtraAdvancedPostings = advancedPostingsDraft.current?.slice(postings.length) ?? [];
    return [...postings, ...savedExtraAdvancedPostings];
  }

  function currentPostingsForSimpleMode(type: SimpleTransactionType): PostingInput[] {
    const [firstPosting, secondPosting] = currentPostingsWithFallback(type);

    if (type === "movement") {
      return [
        {
          ...firstPosting,
          amount: makeMovementInputAmount(firstPosting.amount || secondPosting.amount),
          commodity: firstPosting.commodity || secondPosting.commodity,
          unitPrice: "",
          comment: "",
        },
        {
          ...secondPosting,
          amount: "",
          commodity: "",
          unitPrice: "",
        },
      ];
    }

    return [
      firstPosting,
      {
        ...secondPosting,
        unitPrice: "",
        comment: "",
      },
    ];
  }

  function nextPostingsForType(type: TransactionType): PostingInput[] {
    if (type === "advanced") return currentPostingsForAdvancedMode();
    if (transactionType === "advanced") return currentPostingsForSimpleMode(type);
    return templatePostings(type);
  }

  function applyTransactionType(type: TransactionType) {
    if (transactionType === "advanced" && type !== "advanced") {
      advancedPostingsDraft.current = currentPostingsWithFallback("advanced");
    }

    const postings = nextPostingsForType(type);

    setTransactionType(type);
    transactionForm.setFieldsValue({
      mode: type,
      postings,
    });
  }

  function openCreateTransaction() {
    advancedPostingsDraft.current = null;
    setEditingTransaction(null);
    setTransactionType("movement");
    transactionForm.setFieldsValue({
      ...emptyTransaction,
      mode: "movement",
      date: todayJournalDate(),
      postings: templatePostings("movement"),
    });
    setTransactionModalOpen(true);
  }

  function openEditTransaction(transaction: JournalTransaction) {
    advancedPostingsDraft.current = null;
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
