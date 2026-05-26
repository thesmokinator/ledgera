import {
  Button,
  Card,
  Modal,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import {
  ClearOutlined,
  CopyOutlined,
  ReloadOutlined,
} from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import dayjs from "dayjs";
import { useTranslation } from "react-i18next";
import type { LogEntry } from "./types";
import styles from "./LogsRoute.module.css";

function levelColor(level: LogEntry["level"]): string {
  if (level === "error") return "red";
  if (level === "warn") return "gold";
  return "blue";
}

function formatLogEntry(entry: LogEntry): string {
  const timestamp = dayjs(entry.ts).format("YYYY-MM-DD HH:mm:ss");
  return `[${timestamp}] ${entry.level.toUpperCase()} ${entry.code}\n\n${entry.message}`;
}

export function LogsRoute() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [modal, modalContextHolder] = Modal.useModal();

  const logsQuery = useQuery({
    queryKey: ["logs"],
    queryFn: () => invoke<LogEntry[]>("get_logs"),
    retry: false,
  });

  const clearLogsMutation = useMutation({
    mutationFn: () => invoke<void>("clear_logs"),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["logs"] }),
  });

  const logs = logsQuery.data ?? [];

  function copyLogEntry(entry: LogEntry) {
    navigator.clipboard.writeText(formatLogEntry(entry));
  }

  function confirmClear() {
    modal.confirm({
      title: t("logs.clear_all"),
      content: t("logs.clear_confirmation"),
      okText: t("logs.clear_confirm"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      centered: true,
      onOk: () => clearLogsMutation.mutate(),
    });
  }

  return (
    <Space orientation="vertical" size={24} className="content-stack">
      {modalContextHolder}
      <Card
        className={styles.card}
        title={t("logs.title")}
        extra={(
          <Space>
            <Button
              icon={<ReloadOutlined />}
              onClick={() => queryClient.invalidateQueries({ queryKey: ["logs"] })}
            >
              {t("common.refresh")}
            </Button>
            <Button
              danger
              icon={<ClearOutlined />}
              disabled={logs.length === 0}
              onClick={confirmClear}
            >
              {t("logs.clear_all")}
            </Button>
          </Space>
        )}
      >
        <Table<LogEntry>
          dataSource={logs}
          rowKey={(entry) => `${entry.ts}-${entry.code}`}
          loading={logsQuery.isFetching}
          pagination={{ pageSize: 12 }}
          scroll={{ x: 900 }}
          columns={[
            {
              title: t("logs.timestamp"),
              dataIndex: "ts",
              width: 180,
              render: (ts: string) =>
                dayjs(ts).format("YYYY-MM-DD HH:mm:ss"),
            },
            {
              title: t("logs.level"),
              dataIndex: "level",
              width: 96,
              render: (level: LogEntry["level"]) => (
                <Tag color={levelColor(level)}>{level.toUpperCase()}</Tag>
              ),
            },
            {
              title: t("logs.code"),
              dataIndex: "code",
              width: 220,
              ellipsis: true,
            },
            {
              title: t("logs.message"),
              dataIndex: "message",
              ellipsis: true,
              render: (message: string) => (
                <Tooltip
                  overlayClassName={styles.message_tooltip}
                  title={<pre className={styles.message_preview}>{message}</pre>}
                >
                  <Typography.Text className={styles.message} ellipsis>
                    {message}
                  </Typography.Text>
                </Tooltip>
              ),
            },
            {
              width: 48,
              render: (_, entry) => (
                <Button
                  type="text"
                  size="small"
                  icon={<CopyOutlined />}
                  aria-label={t("logs.copy")}
                  title={t("logs.copy")}
                  onClick={(e) => {
                    e.stopPropagation();
                    copyLogEntry(entry);
                  }}
                />
              ),
            },
          ]}
        />
      </Card>
    </Space>
  );
}
