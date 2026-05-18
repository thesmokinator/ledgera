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
  FolderOutlined,
  SettingOutlined,
  UploadOutlined,
} from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import packageJson from "../../package.json";
import { formatCount, formatFileSize } from "../utils/format";
import { parseError } from "../utils/error";
import type { AppSettings, HledgerStatus, JournalSummary } from "./types";
import styles from "./SettingsRoute.module.css";

const projectRepositoryUrl = packageJson.repository.url.replace(/\.git$/, "");

function CardTitle({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <Space className={styles.sectionTitle}>
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
    <Space.Compact block className={styles.pathInputGroup}>
      <Input
        value={value}
        onChange={(event) => onChange?.(event.target.value)}
        placeholder={placeholder}
      />
      {statusAddon ? <div className={styles.pathInputAddon}>{statusAddon}</div> : null}
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
  onValuesChange,
}: {
  form: FormInstance<AppSettings>;
  initialValues: AppSettings;
  commodityOptions: { value: string }[];
  hledgerStatus: HledgerStatus | undefined;
  journalSummary: JournalSummary | undefined;
  journalError: string | null;
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
            label={t("settings.journalPath")}
            name="journalPath"
            rules={[{ required: true, message: t("settings.journalPathRequired") }]}
          >
            <PathInput
              placeholder={t("settings.journalPathPlaceholder")}
              pickerTitle={t("settings.pickJournalFile")}
            />
          </Form.Item>

          {journalError ? (
            <Typography.Text type="danger">
              {parseError(journalError, t)}
            </Typography.Text>
          ) : stats ? (
            <div className={styles.statsGrid}>
              <div className={styles.statsItem}>
                <span className={styles.statsLabel}>{t("settings.statsTransactions")}</span>
                <span className={styles.statsValue}>{formatCount(stats.transactions)}</span>
              </div>
              <div className={styles.statsItem}>
                <span className={styles.statsLabel}>{t("settings.statsAccounts")}</span>
                <span className={styles.statsValue}>{formatCount(stats.accounts)}</span>
              </div>
              <div className={styles.statsItem}>
                <span className={styles.statsLabel}>{t("settings.statsCommodities")}</span>
                <span className={styles.statsValue}>{formatCount(stats.commodities)}</span>
              </div>
              <div className={styles.statsItem}>
                <span className={styles.statsLabel}>{t("settings.statsDateRange")}</span>
                <span className={styles.statsValue}>
                  {stats.dateMin && stats.dateMax
                    ? `${stats.dateMin} → ${stats.dateMax}`
                    : "-"}
                </span>
              </div>
              <div className={styles.statsItem}>
                <span className={styles.statsLabel}>{t("settings.statsFiles")}</span>
                <span className={styles.statsValue}>{formatCount(stats.fileCount)}</span>
              </div>
              <div className={styles.statsItem}>
                <span className={styles.statsLabel}>{t("settings.statsFileSize")}</span>
                <span className={styles.statsValue}>{formatFileSize(stats.fileSize)}</span>
              </div>
            </div>
          ) : (
            <Typography.Text type="secondary">
              {t("settings.configureJournalTitle")}
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
              <Tag color="error">{t("settings.hledgerNotAvailable")}</Tag>
            ) : (
              <Tag>{t("settings.hledgerNotConfigured")}</Tag>
            )
          }
        >
          <Form.Item
            label={t("settings.hledgerExecutable")}
            name="hledgerPath"
          >
            <PathInput
              placeholder={
                hledgerStatus?.resolvedPath || t("settings.hledgerExecutablePlaceholder")
              }
              pickerTitle={t("settings.pickHledgerExecutable")}
            />
          </Form.Item>
        </Card>

        {/* ── Preferences ──────────────────────────── */}
        <Card
          className={styles.card}
          title={<CardTitle icon={<SettingOutlined />} label={t("settings.preferences")} />}
        >
          <div className={styles.preferencesStack}>
            <Form.Item label={t("settings.defaultCommodity")} name="defaultCommodity">
              <AutoComplete
                options={commodityOptions}
                placeholder={t("settings.defaultCommodityPlaceholder")}
                filterOption
              />
            </Form.Item>
            <Form.Item label={t("settings.theme")} name="theme" rules={[{ required: true }]}>
              <Select
                options={[
                  { value: "system", label: t("settings.themeSystem") },
                  { value: "dark", label: t("settings.themeDark") },
                  { value: "light", label: t("settings.themeLight") },
                ]}
              />
            </Form.Item>
            <div className={styles.developerSettings}>
              <div>
                <Typography.Text strong>{t("settings.developerOptions")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.advancedModeHelp")}
                </Typography.Paragraph>
              </div>
              <Form.Item name="powerUser" valuePropName="checked" noStyle>
                <Switch />
              </Form.Item>
            </div>
            <div className={styles.developerSettings}>
              <div>
                <Typography.Text strong>{t("settings.fetchPrices")}</Typography.Text>
                <Typography.Paragraph type="secondary">
                  {t("settings.fetchPricesHelp")}
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
                    className={styles.textAreaSetting}
                    label={t("settings.commoditySymbols")}
                    name="commoditySymbols"
                    help={t("settings.commoditySymbolsHelp")}
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
              className={styles.textAreaSetting}
              label={t("settings.excludeBalances")}
              name="excludeBalances"
              help={t("settings.excludeBalancesHelp")}
            >
              <Input.TextArea
                rows={3}
                placeholder="assets:investments:xeon"
                style={{ fontFamily: "monospace", fontSize: 13 }}
              />
            </Form.Item>
            <Form.Item
              className={styles.textAreaSetting}
              label={t("settings.includeInvestments")}
              name="includeInvestments"
              help={t("settings.includeInvestmentsHelp")}
            >
              <Input.TextArea
                rows={3}
                placeholder="assets:investments:xeon"
                style={{ fontFamily: "monospace", fontSize: 13 }}
              />
            </Form.Item>
          </div>
        </Card>

        {/* ── Footer ───────────────────────────────── */}
        <div className={styles.footer}>
          <Typography.Text type="secondary">
            {t("settings.version", { version: packageJson.version })}
          </Typography.Text>
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
