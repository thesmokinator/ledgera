import { SearchOutlined } from "@ant-design/icons";
import { Button, Layout, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { newTransactionShortcut, spotlightShortcut } from "../utils/shortcut";

export function AppHeader({
  title,
  disableActions,
  onCreateTransaction,
  onOpenSearch,
}: {
  title: string;
  disableActions: boolean;
  onCreateTransaction: () => void;
  onOpenSearch: () => void;
}) {
  const { t } = useTranslation();

  return (
    <Layout.Header className="app-header">
      <div className="titlebar-drag" data-tauri-drag-region>
        <Typography.Title level={3}>{title}</Typography.Title>
      </div>
      <div className="header-actions">
        <button
          type="button"
          className="search_trigger"
          disabled={disableActions}
          onClick={onOpenSearch}
          title={t("common.search")}
          aria-label={t("common.search")}
        >
          <SearchOutlined className="search_trigger_icon" />
          <span className="search_trigger_label">Search journal…</span>
          <span className="search_trigger_shortcut">{spotlightShortcut()}</span>
        </button>
        <Button type="primary" disabled={disableActions} onClick={onCreateTransaction}>
          {t("transactions.new_transaction")} ({newTransactionShortcut()})
        </Button>
      </div>
    </Layout.Header>
  );
}
