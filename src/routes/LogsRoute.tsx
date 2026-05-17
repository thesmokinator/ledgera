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

const levelColors: Record<LogEntry["level"], string> = {
  info: "blue",
  warn: "orange",
  error: "red",
};

const levelLabels: Record<LogEntry["level"], string> = {
  info: "INFO",
  warn: "WARN",
  error: "ERROR",
};

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
    navigator.clipboard.writeText(JSON.stringify(entry, null, 2));
  }

  function confirmClear() {
    modal.confirm({
      title: t("logs.clearAll"),
      content: t("logs.clearConfirmation"),
      okText: t("logs.clearConfirm"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      centered: true,
      onOk: () => clearLogsMutation.mutate(),
    });
  }

  return (
    <Space direction="vertical" size={24} className="content-stack">
      {modalContextHolder}
      <Card
        className="settings-card"
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
              {t("logs.clearAll")}
            </Button>
          </Space>
        )}
      >
        <Table<LogEntry>
          dataSource={logs}
          rowKey={(entry) => `${entry.ts}-${entry.code}`}
          loading={logsQuery.isFetching}
          pagination={{ pageSize: 12 }}
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
              width: 80,
              render: (level: LogEntry["level"]) => (
                <Tag color={levelColors[level]}>{levelLabels[level]}</Tag>
              ),
            },
            {
              title: t("logs.code"),
              dataIndex: "code",
              width: 200,
              ellipsis: true,
            },
            {
              title: t("logs.message"),
              dataIndex: "message",
              ellipsis: true,
            },
            {
              title: t("logs.details"),
              dataIndex: "details",
              width: 280,
              ellipsis: true,
              render: (details?: string) =>
                details ? (
                  <Tooltip title={details}>
                    <Typography.Text
                      style={{ maxWidth: 260 }}
                      ellipsis
                    >
                      {details}
                    </Typography.Text>
                  </Tooltip>
                ) : (
                  "—"
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
