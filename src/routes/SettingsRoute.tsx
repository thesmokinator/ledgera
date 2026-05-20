import {
  AutoComplete,
  Button,
  Card,
  Form,
  Input,
  Select,
  Space,
  Switch,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import type { FormInstance } from "antd";
import {
  CodeOutlined,
  DownloadOutlined,
  FolderOutlined,
  InfoCircleOutlined,
  SettingOutlined,
  UploadOutlined,
} from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import packageJson from "../../package.json";
import { formatCount, formatFileSize } from "../utils/format";
import { parseError } from "../utils/error";
import type { AppSettings, HledgerStatus, JournalSummary, UpdateStatus } from "./types";
import styles from "./SettingsRoute.module.css";

const projectRepositoryUrl = packageJson.repository.url.replace(/\.git$/, "");

function CardTitle({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <Space className={styles.section_title}>
      {icon}
      <span>{label}</span>
    </Space>
  );
}

function PathInput({
  value,
  onChange,
  placeholder,
  pickerTitle,
  statusAddon,
}: {
  value?: string;
  onChange?: (value: string) => void;
  placeholder: string;
  pickerTitle: string;
  statusAddon?: ReactNode;
}) {
  async function selectFile() {
    const selected = await open({
      multiple: false,
      directory: false,
      title: pickerTitle,
    });
    if (typeof selected === "string") {
      onChange?.(selected);
    }
  }

  return (
    <Space.Compact block className={styles.path_input_group}>
      <Input
        value={value}
        onChange={(event) => onChange?.(event.target.value)}
        placeholder={placeholder}
      />
      {statusAddon ? <div className={styles.path_input_addon}>{statusAddon}</div> : null}
      <Tooltip title={pickerTitle}>
        <Button icon={<UploadOutlined />} onClick={selectFile} />
      </Tooltip>
    </Space.Compact>
  );
}

export function SettingsRoute({
  form,
  initialValues,
  commodityOptions,
  hledgerStatus,
  journalSummary,
  journalError,
  updateStatus,
  isCheckingForUpdates,
  onCheckForUpdates,
  onValuesChange,
}: {
  form: FormInstance<AppSettings>;
  initialValues: AppSettings;
  commodityOptions: { value: string }[];
  hledgerStatus: HledgerStatus | undefined;
  journalSummary: JournalSummary | undefined;
  journalError: string | null;
  updateStatus: UpdateStatus | undefined;
  isCheckingForUpdates: boolean;
  onCheckForUpdates: () => void;
  onValuesChange: (changed: Partial<AppSettings>, values: AppSettings) => void;
}) {
  const { t } = useTranslation();
  const licenseUrl = `${projectRepositoryUrl}/blob/main/LICENSE.md`;

  const stats = journalSummary
    ? {
      transactions: journalSummary.transactions.length,
      commodities: journalSummary.commodities.length,
      accounts: new Set(
        journalSummary.transactions.flatMap((tx) =>
          tx.postings.map((p) => p.account.toLowerCase()),
        ),
      ).size,
      dateMin: journalSummary.transactions.length
        ? journalSummary.transactions[journalSummary.transactions.length - 1].date.slice(0, 7)
        : null,
      dateMax: journalSummary.transactions.length
        ? journalSummary.transactions[0].date.slice(0, 7)
        : null,
      fileCount: journalSummary.fileCount,
      fileSize: journalSummary.totalSizeBytes,
    }
    : null;

  return (
    <Form<AppSettings>
      form={form}
      layout="vertical"
      initialValues={initialValues}
      onValuesChange={onValuesChange}
    >
      <Space direction="vertical" size={24} className="content-stack">
        {/* ── Journal ──────────────────────────────── */}
        <Card
          className={styles.card}
          title={<CardTitle icon={<FolderOutlined />} label={t("settings.journal")} />}
        >
          <Form.Item
            label={t("settings.journal_path")}
            name="journalPath"
            rules={[{ required: true, message: t("settings.journal_path_required") }]}
          >
            <PathInput
              placeholder={t("settings.journal_path_placeholder")}
              pickerTitle={t("settings.pick_journal_file")}
            />
          </Form.Item>

          {journalError ? (
            <Typography.Text type="danger">
              {parseError(journalError, t)}
            </Typography.Text>
          ) : stats ? (
            <div className={styles.stats_grid}>
              <div className={styles.stats_item}>
                <span className={styles.stats_label}>{t("settings.stats_transactions")}</span>
                <span className={styles.stats_value}>{formatCount(stats.transactions)}</span>
              </div>
              <div className={styles.stats_item}>
                <span className={styles.stats_label}>{t("settings.stats_accounts")}</span>
                <span className={styles.stats_value}>{formatCount(stats.accounts)}</span>
              </div>
              <div className={styles.stats_item}>
                <span className={styles.stats_label}>{t("settings.stats_commodities")}</span>
                <span className={styles.stats_value}>{formatCount(stats.commodities)}</span>
              </div>
              <div className={styles.stats_item}>
                <span className={styles.stats_label}>{t("settings.stats_date_range")}</span>
                <span className={styles.stats_value}>
                  {stats.dateMin && stats.dateMax
                    ? `${stats.dateMin} → ${stats.dateMax}`
                    : "-"}
                </span>
              </div>
              <div className={styles.stats_item}>
                <span className={styles.stats_label}>{t("settings.stats_files")}</span>
                <span className={styles.stats_value}>{formatCount(stats.fileCount)}</span>
              </div>
              <div className={styles.stats_item}>
                <span className={styles.stats_label}>{t("settings.stats_file_size")}</span>
                <span className={styles.stats_value}>{formatFileSize(stats.fileSize)}</span>
              </div>
            </div>
          ) : (
            <Typography.Text type="secondary">
              {t("settings.configure_journal_title")}
            </Typography.Text>
          )}
        </Card>

        {/* ── hledger ─────────────────────────────── */}
        <Card
          className={styles.card}
          title={<CardTitle icon={<CodeOutlined />} label={t("settings.hledger")} />}
          extra={
            hledgerStatus?.available ? (
              <Tag color="success">
                {hledgerStatus.version || ""}
              </Tag>
            ) : hledgerStatus ? (
              <Tag color="error">{t("settings.hledger_not_available")}</Tag>
            ) : (
              <Tag>{t("settings.hledger_not_configured")}</Tag>
            )
          }
        >
          <Form.Item
            label={t("settings.hledger_executable")}
            name="hledgerPath"
          >
            <PathInput
              placeholder={
                hledgerStatus?.resolvedPath || t("settings.hledger_executable_placeholder")
              }
              pickerTitle={t("settings.pick_hledger_executable")}
            />
          </Form.Item>
        </Card>

        {/* ── Preferences ──────────────────────────── */}
        <Card
          className={styles.card}
          title={<CardTitle icon={<SettingOutlined />} label={t("settings.preferences")} />}
        >
          <div className={styles.preferences_stack}>
            <Form.Item label={t("settings.default_commodity")} name="defaultCommodity">
              <AutoComplete
                options={commodityOptions}
                placeholder={t("settings.default_commodity_placeholder")}
                filterOption
              />
            </Form.Item>
            <Form.Item label={t("settings.theme")} name="theme" rules={[{ required: true }]}>
              <Select
                options={[
                  { value: "system", label: t("settings.theme_system") },
                  { value: "dark", label: t("settings.theme_dark") },
                  { value: "light", label: t("settings.theme_light") },
                ]}
              />
            </Form.Item>
            <div className={styles.developer_settings}>
              <div>
                <Typography.Text strong>{t("settings.prefill_postings")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.prefill_postings_help")}
                </Typography.Paragraph>
              </div>
              <Form.Item name="prefillPostings" valuePropName="checked" noStyle>
                <Switch />
              </Form.Item>
            </div>
            <div className={styles.developer_settings}>
              <div>
                <Typography.Text strong>{t("settings.fetch_prices")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.fetch_prices_help")}
                </Typography.Paragraph>
              </div>
              <Form.Item name="fetchPrices" valuePropName="checked" noStyle>
                <Switch />
              </Form.Item>
            </div>
            <Form.Item noStyle shouldUpdate={(prev, cur) => prev.fetchPrices !== cur.fetchPrices}>
              {({ getFieldValue }) =>
                getFieldValue("fetchPrices") ? (
                  <Form.Item
                    className={styles.text_area_setting}
                    label={t("settings.commodity_symbols")}
                    name="commoditySymbols"
                    help={t("settings.commodity_symbols_help")}
                  >
                    <Input.TextArea
                      rows={3}
                      placeholder={"VWCE=VWCE.DE\nXEON=XEON.DE"}
                      style={{ fontFamily: "monospace", fontSize: 13 }}
                    />
                  </Form.Item>
                ) : null
              }
            </Form.Item>
            <Form.Item
              className={styles.text_area_setting}
              label={t("settings.exclude_balances")}
              name="excludeBalances"
              help={t("settings.exclude_balances_help")}
            >
              <Input.TextArea
                rows={3}
                placeholder="assets:investments:xeon"
                style={{ fontFamily: "monospace", fontSize: 13 }}
              />
            </Form.Item>
            <Form.Item
              className={styles.text_area_setting}
              label={t("settings.include_investments")}
              name="includeInvestments"
              help={t("settings.include_investments_help")}
            >
              <Input.TextArea
                rows={3}
                placeholder="assets:investments:xeon"
                style={{ fontFamily: "monospace", fontSize: 13 }}
              />
            </Form.Item>
            <div className={styles.developer_settings}>
              <div>
                <Typography.Text strong>{t("settings.developer_options")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.advanced_mode_help")}
                </Typography.Paragraph>
              </div>
              <Form.Item name="powerUser" valuePropName="checked" noStyle>
                <Switch />
              </Form.Item>
            </div>
          </div>
        </Card>

        {/* ── App ──────────────────────────────────── */}
        <Card
          className={styles.card}
          title={<CardTitle icon={<InfoCircleOutlined />} label={t("settings.app")} />}
          extra={updateStatus?.available ? <Tag color="warning">{t("settings.update_available_short")}</Tag> : null}
        >
          <div className={styles.app_update_panel}>
            <div>
              <Typography.Text strong>{t("settings.version_label")}</Typography.Text>
              <Typography.Paragraph type="secondary">
                {updateStatus?.currentVersion ?? packageJson.version}
              </Typography.Paragraph>
            </div>
            <div>
              <Typography.Text strong>{t("settings.updates")}</Typography.Text>
              <Typography.Paragraph type={updateStatus?.error ? "danger" : "secondary"}>
                {updateStatus?.error
                  ? t("settings.update_check_failed")
                  : updateStatus?.available
                    ? t("settings.update_available", { version: updateStatus.latestVersion })
                    : t("settings.up_to_date")}
              </Typography.Paragraph>
              {updateStatus?.checkedAt ? (
                <Typography.Text type="secondary" className={styles.update_checked_at}>
                  {t("settings.last_checked", { date: new Date(updateStatus.checkedAt).toLocaleString() })}
                </Typography.Text>
              ) : null}
            </div>
            <Space wrap>
              <Button
                icon={<DownloadOutlined />}
                loading={isCheckingForUpdates}
                onClick={onCheckForUpdates}
              >
                {t("settings.check_for_updates")}
              </Button>
              {updateStatus?.releaseUrl ? (
                <Button href={updateStatus.releaseUrl} target="_blank">
                  {t("settings.view_release")}
                </Button>
              ) : null}
            </Space>
          </div>
        </Card>

        {/* ── Footer ───────────────────────────────── */}
        <div className={styles.footer}>
          <Space wrap>
            <Button size="small" href={projectRepositoryUrl} target="_blank">
              {t("settings.repository")}
            </Button>
            <Button size="small" href={licenseUrl} target="_blank">
              {t("settings.license", { license: packageJson.license })}
            </Button>
          </Space>
        </div>
      </Space>
    </Form>
  );
}
