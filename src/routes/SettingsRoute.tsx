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
  DeleteOutlined,
  DownloadOutlined,
  FolderOutlined,
  InfoCircleOutlined,
  PlusOutlined,
  ReadOutlined,
  SettingOutlined,
  UploadOutlined,
} from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import packageJson from "../../package.json";
import { formatCount, formatFileSize } from "../utils/format";
import { parseError } from "../utils/error";
import { supportedLanguages } from "../utils/language";
import type { AppSettings, HledgerStatus, JournalSummary, UpdateStatus } from "./types";
import styles from "./SettingsRoute.module.css";

const projectRepositoryUrl = packageJson.repository.url.replace(/\.git$/, "");
const projectWikiUrl = `${projectRepositoryUrl}/wiki`;

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
  accountOptions,
  hledgerStatus,
  journalSummary,
  journalError,
  updateStatus,
  isCheckingForUpdates,
  updateCheckerEnabled,
  onCheckForUpdates,
  onValuesChange,
}: {
  form: FormInstance<AppSettings>;
  initialValues: AppSettings;
  commodityOptions: { value: string }[];
  accountOptions: { value: string }[];
  hledgerStatus: HledgerStatus | undefined;
  journalSummary: JournalSummary | undefined;
  journalError: string | null;
  updateStatus: UpdateStatus | undefined;
  isCheckingForUpdates: boolean;
  updateCheckerEnabled: boolean;
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
      <Space orientation="vertical" size={24} className="content-stack">
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
            <Form.Item label={t("settings.language")} name="language" rules={[{ required: true }]}>
              <Select
                options={supportedLanguages.map((language) => ({
                  value: language.value,
                  label: t(language.labelKey),
                }))}
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

            <Form.Item
              label={t("settings.exclude_balances")}
              help={t("settings.exclude_balances_help")}
            >
              <Form.List name="excludeBalances">
                {(fields, { add, remove }) => (
                  <Space direction="vertical" style={{ width: "100%" }}>
                    {fields.map((field) => (
                      <Space key={field.key} align="start" style={{ width: "100%" }}>
                        <Form.Item
                          name={field.name}
                          rules={[{ required: true, whitespace: true }]}
                          noStyle
                        >
                          <AutoComplete
                            options={accountOptions}
                            placeholder="assets:investments"
                            style={{ width: 280 }}
                            filterOption
                          />
                        </Form.Item>
                        <Button
                          danger
                          icon={<DeleteOutlined />}
                          aria-label={t("settings.remove_symbol_mapping")}
                          onClick={() => remove(field.name)}
                        />
                      </Space>
                    ))}
                    <Button
                      icon={<PlusOutlined />}
                      onClick={() => add("")}
                      style={{ marginBottom: 8 }}
                    >
                      {t("settings.add_account")}
                    </Button>
                  </Space>
                )}
              </Form.List>
            </Form.Item>
            <Form.Item
              label={t("settings.include_investments")}
              help={t("settings.include_investments_help")}
            >
              <Form.List name="includeInvestments">
                {(fields, { add, remove }) => (
                  <Space direction="vertical" style={{ width: "100%" }}>
                    {fields.map((field) => (
                      <Space key={field.key} align="start" style={{ width: "100%" }}>
                        <Form.Item
                          name={field.name}
                          rules={[{ required: true, whitespace: true }]}
                          noStyle
                        >
                          <AutoComplete
                            options={accountOptions}
                            placeholder="assets:investments:xeon"
                            style={{ width: 280 }}
                            filterOption
                          />
                        </Form.Item>
                        <Button
                          danger
                          icon={<DeleteOutlined />}
                          aria-label={t("settings.remove_symbol_mapping")}
                          onClick={() => remove(field.name)}
                        />
                      </Space>
                    ))}
                    <Button
                      icon={<PlusOutlined />}
                      onClick={() => add("")}
                      style={{ marginBottom: 8 }}
                    >
                      {t("settings.add_account")}
                    </Button>
                  </Space>
                )}
              </Form.List>
            </Form.Item>
          </div>
        </Card>

        {/* ── Modules ──────────────────────────────── */}
        <Card
          className={styles.card}
          title={<CardTitle icon={<SettingOutlined />} label={t("settings.modules")} />}
        >
          <div className={styles.preferences_stack}>
            <Typography.Paragraph type="secondary">
              {t("settings.modules_help")}
            </Typography.Paragraph>
            <div className={styles.developer_settings}>
              <div>
                <Typography.Text strong>{t("settings.module_market_prices")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.fetch_prices_help")}
                </Typography.Paragraph>
              </div>
              <Form.Item name={["modules", "marketPrices", "enabled"]} valuePropName="checked" noStyle>
                <Switch />
              </Form.Item>
            </div>
            <Form.Item
              noStyle
              shouldUpdate={(prev, cur) =>
                prev.modules?.marketPrices?.enabled !== cur.modules?.marketPrices?.enabled
              }
            >
              {({ getFieldValue }) =>
                getFieldValue(["modules", "marketPrices", "enabled"]) ? (
                  <Form.Item
                    label={t("settings.commodity_symbols")}
                    help={t("settings.commodity_symbols_help")}
                  >
                    <Form.List name="commoditySymbols">
                      {(fields, { add, remove }) => (
                        <Space orientation="vertical" style={{ width: "100%" }}>
                          {fields.map((field) => (
                            <Space key={field.key} align="start">
                              <Form.Item
                                name={[field.name, "commodity"]}
                                rules={[{ required: true, whitespace: true, message: t("settings.commodity_required") }]}
                                noStyle
                              >
                                <Input placeholder="VWCE" style={{ width: 140 }} />
                              </Form.Item>
                              <Form.Item
                                name={[field.name, "yahooSymbol"]}
                                rules={[{ required: true, whitespace: true, message: t("settings.yahoo_symbol_required") }]}
                                noStyle
                              >
                                <Input placeholder="VWCE.DE" style={{ width: 140 }} />
                              </Form.Item>
                              <Button
                                danger
                                icon={<DeleteOutlined />}
                                aria-label={t("settings.remove_symbol_mapping")}
                                onClick={() => remove(field.name)}
                              />
                            </Space>
                          ))}
                          <Button
                            icon={<PlusOutlined />}
                            onClick={() => add({ commodity: "", yahooSymbol: "" })}
                            style={{ marginBottom: 8 }}
                          >
                            {t("settings.add_symbol_mapping")}
                          </Button>
                        </Space>
                      )}
                    </Form.List>
                  </Form.Item>
                ) : null
              }
            </Form.Item>
            <div className={styles.developer_settings}>
              <div>
                <Typography.Text strong>{t("settings.module_developer_tools")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.advanced_mode_help")}
                </Typography.Paragraph>
              </div>
              <Form.Item name={["modules", "developerTools", "enabled"]} valuePropName="checked" noStyle>
                <Switch />
              </Form.Item>
            </div>
            <div className={styles.developer_settings}>
              <div>
                <Typography.Text strong>{t("settings.module_update_checker")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.module_update_checker_help")}
                </Typography.Paragraph>
              </div>
              <Form.Item name={["modules", "updateChecker", "enabled"]} valuePropName="checked" noStyle>
                <Switch />
              </Form.Item>
            </div>
            <div className={styles.developer_settings}>
              <div>
                <Space size="small">
                  <Typography.Text strong>{t("settings.module_git_sync")}</Typography.Text>
                </Space>
                <Typography.Paragraph type="secondary">
                  {t("settings.module_git_sync_help")}
                </Typography.Paragraph>
              </div>
              <Form.Item name={["modules", "gitSync", "enabled"]} valuePropName="checked" noStyle>
                <Switch />
              </Form.Item>
            </div>
            <Form.Item
              noStyle
              shouldUpdate={(prev, cur) =>
                prev.modules?.gitSync?.enabled !== cur.modules?.gitSync?.enabled
              }
            >
              {({ getFieldValue }) =>
                getFieldValue(["modules", "gitSync", "enabled"]) ? (
                  <Form.Item
                    label={t("settings.module_git_sync_commit_message")}
                    name={["modules", "gitSync", "commitMessage"]}
                    rules={[{ required: true, whitespace: true, message: t("settings.module_git_sync_commit_message_required") }]}
                  >
                    <Input maxLength={200} showCount />
                  </Form.Item>
                ) : null
              }
            </Form.Item>
            <div className={styles.developer_settings}>
              <div>
                <Typography.Text strong>{t("settings.module_auto_generate_recurring")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.module_auto_generate_recurring_help")}
                </Typography.Paragraph>
              </div>
              <Form.Item name={["modules", "autoGenerateRecurring"]} valuePropName="checked" noStyle>
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
              {updateCheckerEnabled && (
                <>
                  <Typography.Paragraph type={updateStatus?.error ? "danger" : "secondary"}>
                    {updateStatus?.error
                      ? t("settings.update_check_failed")
                      : updateStatus?.available
                        ? t("settings.update_available", { version: updateStatus.latestVersion })
                        : t("settings.up_to_date")}
                  </Typography.Paragraph>
                  {updateStatus?.checkedAt && (
                    <Typography.Text type="secondary" className={styles.update_checked_at}>
                      {t("settings.last_checked", { date: new Date(updateStatus.checkedAt).toLocaleString() })}
                    </Typography.Text>
                  )}
                </>
              )}
              {!updateCheckerEnabled && (
                <Typography.Paragraph type="secondary">
                  {t("settings.update_check_disabled")}
                </Typography.Paragraph>
              )}
            </div>
            {updateCheckerEnabled && (
              <Space wrap>
                <Button
                  icon={<DownloadOutlined />}
                  loading={isCheckingForUpdates}
                  onClick={onCheckForUpdates}
                >
                  {t("settings.check_for_updates")}
                </Button>
                {updateStatus?.releaseUrl && (
                  <Button href={updateStatus.releaseUrl} target="_blank">
                    {t("settings.view_release")}
                  </Button>
                )}
              </Space>
            )}
          </div>
        </Card>

        {/* ── Footer ───────────────────────────────── */}
        <div className={styles.footer}>
          <Button
            size="small"
            icon={<ReadOutlined />}
            onClick={() => { void openUrl(projectWikiUrl); }}
          >
            {t("settings.documentation")}
          </Button>
          <div className={styles.footer_links}>
            <Button size="small" onClick={() => { void openUrl(projectRepositoryUrl); }}>
              {t("settings.repository")}
            </Button>
            <Button size="small" onClick={() => { void openUrl(licenseUrl); }}>
              {t("settings.license", { license: packageJson.license })}
            </Button>
          </div>
        </div>
      </Space>
    </Form>
  );
}
