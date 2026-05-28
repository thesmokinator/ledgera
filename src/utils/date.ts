import dayjs from "dayjs";
import type { Dayjs } from "dayjs";
import type { JournalTransaction } from "../types";

export const journalDateFormat = "YYYY-MM-DD";

export function todayJournalDate(): string {
  return dayjs().format(journalDateFormat);
}

export function isValidJournalDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return false;
  }

  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));
  return (
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
}

export function isSameJournalMonth(date: string, month: Dayjs): boolean {
  const parsedDate = dayjs(date, journalDateFormat, true);
  return parsedDate.isValid() && parsedDate.isSame(month, "month");
}

export function isExecutedTransaction(transaction: JournalTransaction): boolean {
  const parsedDate = dayjs(transaction.date, journalDateFormat, true);
  return parsedDate.isValid() && !parsedDate.isAfter(dayjs(), "day");
}

