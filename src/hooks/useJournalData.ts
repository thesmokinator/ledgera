import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import type { AppSettings, AutocompleteSuggestions, JournalSummary } from "../types";
import { callCommand } from "../utils/command";
import { toAutocompleteOptions } from "../utils/format";

const emptyAutocompleteSuggestions: AutocompleteSuggestions = {
  codes: [],
  descriptions: [],
  accounts: [],
  commodities: [],
  comments: [],
  defaultCommodity: "",
  defaultCashAccount: "",
  defaultExpenseAccount: "",
  defaultIncomeAccount: "",
  defaultTransferAccount: "",
  defaultInvestmentAccount: "",
  defaultInvestmentCommodity: "",
};

export function useJournalData(settings: AppSettings) {
  const hasConfiguredJournal = settings.journalPath.trim().length > 0;

  const transactionsQuery = useQuery({
    queryKey: ["transactions"],
    queryFn: () => callCommand<JournalSummary>("list_transactions"),
    enabled: hasConfiguredJournal,
    retry: false,
    refetchOnMount: true,
  });

  const autocompleteQuery = useQuery({
    queryKey: ["autocomplete-suggestions"],
    queryFn: () => callCommand<AutocompleteSuggestions>("get_autocomplete_suggestions"),
    enabled: hasConfiguredJournal,
    retry: false,
  });

  const autocompleteSuggestions = autocompleteQuery.data ?? emptyAutocompleteSuggestions;
  const codeOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.codes),
    [autocompleteSuggestions.codes],
  );
  const descriptionOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.descriptions),
    [autocompleteSuggestions.descriptions],
  );
  const accountOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.accounts),
    [autocompleteSuggestions.accounts],
  );
  const commodityOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.commodities),
    [autocompleteSuggestions.commodities],
  );
  const commentOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.comments),
    [autocompleteSuggestions.comments],
  );

  return {
    transactionsQuery,
    autocompleteQuery,
    autocompleteSuggestions,
    codeOptions,
    descriptionOptions,
    accountOptions,
    commodityOptions,
    commentOptions,
    defaultCommodity: autocompleteSuggestions.defaultCommodity || "",
    hasConfiguredJournal,
    journalLoadError: transactionsQuery.isError ? String(transactionsQuery.error) : "",
  };
}
