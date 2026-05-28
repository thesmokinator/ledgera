import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppSettings, Notifier } from "../types";
import { callCommand } from "../utils/command";
import { parseError } from "../utils/error";
import { normalizeSettings } from "../utils/settings";

export function useAppSettings({
  messageApi,
  t,
}: {
  messageApi: Notifier;
  t: (key: string, options?: Record<string, unknown>) => string;
}) {
  const queryClient = useQueryClient();

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: async () => normalizeSettings(await callCommand<AppSettings>("get_app_settings")),
  });

  const activeSettings = normalizeSettings(settingsQuery.data);

  const updateSettingsMutation = useMutation({
    mutationFn: (settings: AppSettings) =>
      callCommand<AppSettings>("update_app_settings", {
        settings: normalizeSettings(settings),
      }),
    onSuccess: async (_, variables) => {
      const next = normalizeSettings(variables);
      queryClient.setQueryData(["settings"], next);

      const prev = activeSettings;
      if (prev.journalPath !== next.journalPath) {
        queryClient.resetQueries({ queryKey: ["transactions"] });
        await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      }
      if (prev.hledgerPath !== next.hledgerPath) {
        await queryClient.invalidateQueries({ queryKey: ["hledger-status"] });
      }
      if (prev.defaultCommodity !== next.defaultCommodity) {
        await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      }
      if (prev.excludeBalances !== next.excludeBalances) {
        await queryClient.invalidateQueries({ queryKey: ["balances"] });
      }
      if (prev.includeInvestments !== next.includeInvestments) {
        await queryClient.invalidateQueries({ queryKey: ["investments"] });
      }
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  return {
    settingsQuery,
    activeSettings,
    updateSettingsOnChange: (_: Partial<AppSettings>, values: AppSettings) => {
      updateSettingsMutation.mutate(normalizeSettings(values));
    },
  };
}
