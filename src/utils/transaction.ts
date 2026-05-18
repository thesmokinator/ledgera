import type { JournalTransaction, PostingInput, TransactionInput } from "../types";
import { todayJournalDate } from "./date";

export const emptyTransaction: TransactionInput = {
  date: todayJournalDate(),
  status: "",
  code: "",
  description: "",
  postings: [
    { account: "", amount: "", commodity: "", unitPrice: "", comment: "" },
    { account: "", amount: "", commodity: "", unitPrice: "", comment: "" },
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
          unitPrice: "",
          comment: posting.comment,
        }))
        : emptyTransaction.postings,
  };
}

export function parseAmountValue(amount: string): number {
  const trimmed = amount.trim().replace(",", ".");
  const match = trimmed.match(/(-?[\d.]+)/);
  return match ? parseFloat(match[1]) : 0;
}

export function parseUnitPrice(unitPrice: string): {
  value: number;
  commodity: string;
} {
  const trimmed = unitPrice.trim();
  const value = parseAmountValue(trimmed);
  const commodity = trimmed.replace(/[\d.,\s-]/g, "").trim();
  return { value, commodity };
}

export function autoCalculateBalancingAmounts(
  postings: PostingInput[],
  defaultCommodity: string,
): PostingInput[] {
  const result = postings.map((p) => ({ ...p }));
  const pricedIndex = result.findIndex(
    (p) => p.unitPrice.trim() && p.amount.trim(),
  );
  if (pricedIndex === -1) return result;

  const priced = result[pricedIndex];
  const quantity = parseAmountValue(priced.amount);
  const { value: unitPriceValue, commodity: priceCommodity } =
    parseUnitPrice(priced.unitPrice);
  if (quantity === 0 || unitPriceValue === 0) return result;

  const total = quantity * unitPriceValue;

  const balanceIndex = result.findIndex(
    (p, i) =>
      i !== pricedIndex &&
      !p.unitPrice.trim() &&
      (!p.amount.trim() ||
        p.commodity === priceCommodity ||
        p.commodity === defaultCommodity),
  );
  if (balanceIndex === -1) return result;

  result[balanceIndex] = {
    ...result[balanceIndex],
    amount: (-total).toFixed(2),
    commodity:
      priceCommodity || result[balanceIndex].commodity || defaultCommodity,
  };
  return result;
}
