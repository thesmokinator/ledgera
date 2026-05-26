import { Alert, Button, Card, Input, List, Modal, Space, Statistic, Tag, Typography } from "antd";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AppSettings, GitSyncStatus } from "../types";
import { gitSyncSummary } from "../hooks/useGitSync";
import { formatAppError, parseAppError } from "../utils/error";
import styles from "./SyncRoute.module.css";

type GitFileStatusMeta = {
  labelKey: string;
  color: string;
};

function gitFileStatusMeta(status: string): GitFileStatusMeta {
  if (status.includes("U")) return { labelKey: "sync.file_status_conflict", color: "red" };

  switch (status) {
    case "??":
      return { labelKey: "sync.file_status_new", color: "green" };
    case "A":
      return { labelKey: "sync.file_status_added", color: "green" };
    case "M":
      return { labelKey: "sync.file_status_modified", color: "gold" };
    case "D":
      return { labelKey: "sync.file_status_deleted", color: "red" };
    case "R":
      return { labelKey: "sync.file_status_renamed", color: "blue" };
    case "C":
      return { labelKey: "sync.file_status_copied", color: "cyan" };
    case "!!":
      return { labelKey: "sync.file_status_ignored", color: "default" };
    default:
      return { labelKey: "sync.file_status_changed", color: "default" };
  }
}

export function SyncRoute({
  settings,
  status,
  isChecking,
  isPulling,
  isCommittingAndPushing,
  onRefresh,
  onPull,
  onCommitAndPush,
}: {
  settings: AppSettings;
  status: GitSyncStatus | undefined;
  isChecking: boolean;
  isPulling: boolean;
  isCommittingAndPushing: boolean;
  onRefresh: () => void;
  onPull: () => void;
  onCommitAndPush: (message: string) => void;
}) {
  const { t } = useTranslation();
  const [commitModalOpen, setCommitModalOpen] = useState(false);
  const [commitMessage, setCommitMessage] = useState(settings.modules.gitSync.commitMessage);
  const awaitingCommitAndPush = useRef(false);
  const summary = gitSyncSummary(status);
  const busy = isChecking || isPulling || isCommittingAndPushing;
  const canPull = Boolean(status?.repoFound && status.behind > 0 && !status.dirty && !busy);
  const canCommitAndPush = Boolean(status?.repoFound && status.dirty && status.behind === 0 && !busy);
  const statusError = useMemo(() => status?.error ? parseAppError(status.error) : null, [status?.error]);
  const statusErrorHelpKey = statusError ? `sync.error_help.${statusError.code}` : "";
  const statusErrorHelp = statusError ? t(statusErrorHelpKey) : "";
  const hasStatusErrorHelp = Boolean(statusError && statusErrorHelp !== statusErrorHelpKey);
  const statusErrorMessage = status?.error
    ? statusError
      ? formatAppError(statusError, t, { includeDetails: false })
      : status.error
    : "";
  const statusColor = useMemo(() => {
    if (summary.tone === "success") return "success";
    if (summary.tone === "warning") return "warning";
    if (summary.tone === "danger") return "error";
    return "default";
  }, [summary.tone]);

  function openCommitModal() {
    setCommitMessage(settings.modules.gitSync.commitMessage);
    setCommitModalOpen(true);
  }

  useEffect(() => {
    if (!awaitingCommitAndPush.current || isCommittingAndPushing) return;

    awaitingCommitAndPush.current = false;
    if (status?.repoFound && !status.dirty && !status.error) {
      setCommitModalOpen(false);
    }
  }, [isCommittingAndPushing, status?.dirty, status?.error, status?.repoFound]);

  function submitCommitAndPush() {
    awaitingCommitAndPush.current = true;
    onCommitAndPush(commitMessage);
  }

  return (
    <Space orientation="vertical" size={24} className="content-stack">
      <Card title={t("sync.title")} extra={<Tag color={statusColor}>{t(summary.labelKey, summary.labelOptions)}</Tag>}>
        <Space orientation="vertical" size={16} className={styles.full_width}>
          {status?.error ? (
            <Alert
              type="error"
              showIcon
              message={statusErrorMessage}
              description={hasStatusErrorHelp ? statusErrorHelp : undefined}
            />
          ) : (
            <Typography.Paragraph type="secondary">
              {t("sync.description")}
            </Typography.Paragraph>
          )}
          {!status ? <Alert type="info" showIcon message={t("sync.status_unknown")} description={t("sync.status_unknown_help")} /> : null}

          <div className={styles.stats_grid}>
            <Statistic title={t("sync.branch")} value={status?.branch ?? "-"} />
            <Statistic title={t("sync.remote")} value={status?.remote ?? "-"} />
            <Statistic title={t("sync.ahead")} value={status?.ahead ?? 0} />
            <Statistic title={t("sync.behind")} value={status?.behind ?? 0} />
          </div>

          <div>
            <Typography.Text strong>{t("sync.repository")}</Typography.Text>
            <Typography.Paragraph type="secondary" copyable={Boolean(status?.repoRoot)}>
              {status?.repoRoot ?? t("sync.repository_unknown")}
            </Typography.Paragraph>
          </div>

          <div>
            <Typography.Text strong>{t("sync.last_commit")}</Typography.Text>
            <Typography.Paragraph type="secondary">
              {status?.lastCommit ?? "-"}
            </Typography.Paragraph>
          </div>

          <div>
            <Typography.Text strong>{t("sync.files")}</Typography.Text>
            {status?.files.length ? (
              <List
                className={styles.file_list}
                size="small"
                dataSource={status.files}
                renderItem={(file) => {
                  const fileStatus = gitFileStatusMeta(file.status);

                  return (
                    <List.Item>
                      <Space>
                        <Tag color={fileStatus.color} title={t("sync.file_status_raw", { status: file.status })}>
                          {t(fileStatus.labelKey)}
                        </Tag>
                        <Typography.Text code>{file.path}</Typography.Text>
                      </Space>
                    </List.Item>
                  );
                }}
              />
            ) : (
              <Typography.Paragraph type="secondary">{t("sync.no_changes")}</Typography.Paragraph>
            )}
          </div>

          <Space wrap>
            <Button loading={isChecking} onClick={onRefresh}>{t("sync.refresh")}</Button>
            <Button disabled={!canPull} loading={isPulling} onClick={onPull}>{t("sync.pull")}</Button>
            <Button type="primary" disabled={!canCommitAndPush} loading={isCommittingAndPushing} onClick={openCommitModal}>
              {t("sync.commit_and_push")}
            </Button>
          </Space>
        </Space>
      </Card>

      <Modal
        title={t("sync.commit_modal_title")}
        open={commitModalOpen}
        okText={t("sync.commit_and_push")}
        okButtonProps={{ disabled: !commitMessage.trim() || commitMessage.length > 200 || isCommittingAndPushing }}
        cancelButtonProps={{ disabled: isCommittingAndPushing }}
        confirmLoading={isCommittingAndPushing}
        maskClosable={!isCommittingAndPushing}
        keyboard={!isCommittingAndPushing}
        onCancel={() => setCommitModalOpen(false)}
        onOk={submitCommitAndPush}
      >
        <Space orientation="vertical" className={styles.full_width}>
          <Typography.Paragraph type="secondary">
            {t("sync.commit_modal_description")}
          </Typography.Paragraph>
          <Input
            value={commitMessage}
            maxLength={200}
            showCount
            disabled={isCommittingAndPushing}
            onChange={(event) => setCommitMessage(event.target.value)}
          />
        </Space>
      </Modal>
    </Space>
  );
}
