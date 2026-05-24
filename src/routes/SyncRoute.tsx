import { Alert, Button, Card, Input, List, Modal, Space, Statistic, Tag, Typography } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AppSettings, GitSyncStatus } from "../types";
import { gitSyncSummary } from "../hooks/useGitSync";
import styles from "./SyncRoute.module.css";

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
  const summary = gitSyncSummary(status);
  const busy = isChecking || isPulling || isCommittingAndPushing;
  const canPull = Boolean(status?.repoFound && !status.dirty && !busy);
  const canCommitAndPush = Boolean(status?.repoFound && status.dirty && status.behind === 0 && !busy);
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

  function submitCommitAndPush() {
    onCommitAndPush(commitMessage);
    setCommitModalOpen(false);
  }

  return (
    <Space direction="vertical" size={24} className="content-stack">
      <Card title={t("sync.title")} extra={<Tag color={statusColor}>{t(summary.labelKey, summary.labelOptions)}</Tag>}>
        <Space direction="vertical" size={16} className={styles.full_width}>
          <Typography.Paragraph type="secondary">
            {t("sync.description")}
          </Typography.Paragraph>

          {status?.error ? <Alert type="error" showIcon message={t("sync.status_issue")} description={status.error} /> : null}
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
                renderItem={(file) => (
                  <List.Item>
                    <Space>
                      <Tag>{file.status}</Tag>
                      <Typography.Text code>{file.path}</Typography.Text>
                    </Space>
                  </List.Item>
                )}
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
        okButtonProps={{ disabled: !commitMessage.trim() || commitMessage.length > 200 }}
        confirmLoading={isCommittingAndPushing}
        onCancel={() => setCommitModalOpen(false)}
        onOk={submitCommitAndPush}
      >
        <Space direction="vertical" className={styles.full_width}>
          <Typography.Paragraph type="secondary">
            {t("sync.commit_modal_description")}
          </Typography.Paragraph>
          <Input
            value={commitMessage}
            maxLength={200}
            showCount
            onChange={(event) => setCommitMessage(event.target.value)}
          />
        </Space>
      </Modal>
    </Space>
  );
}
