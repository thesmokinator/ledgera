import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { Notifier, UpdateStatus } from "../types";
import { callCommand } from "../utils/command";

export function useUpdateStatus({
  enabled,
  messageApi,
  t,
}: {
  enabled: boolean;
  messageApi: Notifier;
  t: (key: string, options?: Record<string, unknown>) => string;
}) {
  const queryClient = useQueryClient();

  const updateStatusQuery = useQuery({
    queryKey: ["update-status"],
    queryFn: () => callCommand<UpdateStatus>("check_for_updates", { force: false }),
    enabled,
    retry: false,
  });

  const checkForUpdatesMutation = useMutation({
    mutationFn: () => callCommand<UpdateStatus>("check_for_updates", { force: true }),
    onSuccess: (status) => {
      queryClient.setQueryData(["update-status"], status);
      if (status.available) {
        messageApi.success(t("settings.update_available", { version: status.latestVersion }));
      } else if (status.error) {
        messageApi.error(t("settings.update_check_failed"));
      } else {
        messageApi.success(t("settings.up_to_date"));
      }
    },
    onError: () => messageApi.error(t("settings.update_check_failed")),
  });

  return {
    updateStatus: checkForUpdatesMutation.data ?? updateStatusQuery.data,
    isCheckingForUpdates: updateStatusQuery.isFetching || checkForUpdatesMutation.isPending,
    checkForUpdates: () => checkForUpdatesMutation.mutate(),
  };
}
