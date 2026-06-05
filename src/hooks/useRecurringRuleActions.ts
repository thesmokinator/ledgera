import dayjs from "dayjs";
import { useState } from "react";
import { callCommand } from "../utils/command";
import { normalizeDate } from "../utils/date";
import type { PeriodicRule, PeriodicRuleInput, PeriodicRulesSummary } from "../types";

type RecurringFormValues = Omit<PeriodicRuleInput, 'startDate' | 'endDate'> & {
  _customPeriod?: string;
  startDate?: dayjs.Dayjs;
  endDate?: dayjs.Dayjs;
};

export function useRecurringRuleActions({
  editingRule,
  defaultCommodity,
  onSaved,
}: {
  editingRule: PeriodicRule | null;
  defaultCommodity: string;
  onSaved: () => void;
}) {
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const isEditing = editingRule !== null;

  async function submitRule(values: RecurringFormValues) {
    const { _customPeriod, ...ruleValues } = values;
    const customPeriod = _customPeriod?.trim();
    const periodExpr = customPeriod || values.periodExpr;

    setSaving(true);
    setSaveError(null);
    try {
      const input: PeriodicRuleInput = {
        ...ruleValues,
        periodExpr,
        startDate: normalizeDate(ruleValues.startDate) ?? "",
        endDate: normalizeDate(ruleValues.endDate),
        postings: (ruleValues.postings ?? []).map((posting) => ({
          account: posting.account ?? "",
          amount: posting.amount ?? "",
          commodity: posting.amount?.trim()
            ? (posting.commodity?.trim() || defaultCommodity)
            : (posting.commodity?.trim() || ""),
          unitPrice: posting.unitPrice ?? "",
          comment: posting.comment ?? "",
        })),
      };
      if (isEditing && editingRule) {
        await callCommand<PeriodicRulesSummary>("update_periodic_rule", {
          ruleIdParam: editingRule.ruleId,
          input,
        });
      } else {
        await callCommand<PeriodicRulesSummary>("create_periodic_rule", { input });
      }
      onSaved();
    } catch (err) {
      setSaveError(String(err));
    } finally {
      setSaving(false);
    }
  }

  return {
    submitRule,
    isSaving: saving,
    saveError,
    clearSaveError: () => setSaveError(null),
  };
}
