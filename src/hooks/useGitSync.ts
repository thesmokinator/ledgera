import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppSettings, GitSyncStatus } from "../types";
import { callCommand } from "../utils/command";
import { parseError } from "../utils/error";

type Notifier = {
  success: (content: string) => void;
  error: (content: string) => void;
};

export function gitSyncSummary(status: GitSyncStatus | undefined): {
  tone: "neutral" | "success" | "warning" | "danger";
  labelKey: string;
  labelOptions?: Record<string, unknown>;
} {
  if (!status) return { tone: "neutral", labelKey: "sync.status_unknown" };
  if (status.error || !status.available || !status.repoFound) return { tone: "danger", labelKey: "sync.status_issue" };
  if (status.behind > 0 && status.ahead > 0) return { tone: "warning", labelKey: "sync.status_diverged" };
  if (status.behind > 0) return { tone: "warning", labelKey: "sync.status_behind", labelOptions: { count: status.behind } };
  if (status.ahead > 0) return { tone: "warning", labelKey: "sync.status_ahead", labelOptions: { count: status.ahead } };
  if (status.dirty) return { tone: "warning", labelKey: "sync.status_changes", labelOptions: { count: status.files.length } };
  return { tone: "success", labelKey: "sync.status_synced" };
}

export function useGitSync({
  activeSettings,
  messageApi,
  t,
}: {
  activeSettings: AppSettings;
  messageApi: Notifier;
  t: (key: string, options?: Record<string, unknown>) => string;
}) {
  const queryClient = useQueryClient();
  const enabled = activeSettings.modules.gitSync.enabled && Boolean(activeSettings.journalPath.trim());

  async function invalidateGitAndJournal(status: GitSyncStatus) {
    queryClient.setQueryData(["git-sync-status"], status);
    await queryClient.invalidateQueries({ queryKey: ["transactions"] });
    await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
    await queryClient.invalidateQueries({ queryKey: ["balances"] });
    await queryClient.invalidateQueries({ queryKey: ["investments"] });
  }

  const statusQuery = useQuery({
    queryKey: ["git-sync-status"],
    queryFn: () => callCommand<GitSyncStatus>("git_sync_status"),
    enabled,
    retry: false,
  });

  const pullMutation = useMutation({
    mutationFn: () => callCommand<GitSyncStatus>("git_pull_journal"),
    onSuccess: async (status) => {
      messageApi.success(t("sync.pull_success"));
      await invalidateGitAndJournal(status);
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  const commitAndPushMutation = useMutation({
    mutationFn: (message: string) =>
      callCommand<GitSyncStatus, { message: string }>("git_commit_and_push_journal", { message }),
    onSuccess: async (status) => {
      messageApi.success(t("sync.commit_push_success"));
      await invalidateGitAndJournal(status);
    },
    onError: (error) => messageApi.error(parseError(error, t)),
  });

  return {
    enabled,
    gitSyncStatus: statusQuery.data,
    isCheckingGitSync: statusQuery.isFetching,
    isPulling: pullMutation.isPending,
    isCommittingAndPushing: commitAndPushMutation.isPending,
    refreshGitSyncStatus: () => statusQuery.refetch(),
    pullJournal: () => pullMutation.mutate(),
    commitAndPushJournal: (message: string) => commitAndPushMutation.mutate(message),
  };
}
