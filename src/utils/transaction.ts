import type { JournalTransaction, PostingInput, TransactionInput } from "../types";
import { todayJournalDate } from "./date";

export const emptyTransaction: TransactionInput = {
  date: todayJournalDate(),
  status: "",
  code: "",
  description: "",
  postings: [
    { account: "", amount: "", commodity: "", comment: "" },
    { account: "", amount: "", commodity: "", comment: "" },
  ],
};

export function toTransactionInput(transaction: JournalTransaction): TransactionInput {
  return {
    date: transaction.date,
    status: transaction.status,
    code: transaction.code,
    description: transaction.description,
    postings:
      transaction.postings.length > 0
        ? transaction.postings.map((posting): PostingInput => ({
          account: posting.account,
          amount: posting.amount,
          commodity: posting.commodity,
          comment: posting.comment,
        }))
        : emptyTransaction.postings,
  };
}
