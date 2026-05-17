import {
  AutoComplete,
  Button,
  Card,
  Descriptions,
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
  AppstoreOutlined,
  CodeOutlined,
  UploadOutlined,
} from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import packageJson from "../../package.json";
import type { AppSettings, HledgerStatus } from "./types";

const projectRepositoryUrl = packageJson.repository.url.replace(/\.git$/, "");

function SectionTitle({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <Space className="settings-section-title">
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
    <Space.Compact block className="path-input-group">
      <Input value={value} onChange={(event) => onChange?.(event.target.value)} placeholder={placeholder} />
      {statusAddon ? <div className="path-input-addon">{statusAddon}</div> : null}
      <Tooltip title={pickerTitle}>
        <Button icon={<UploadOutlined />} onClick={selectFile} />
      </Tooltip>
    </Space.Compact>
  );
}

function ApplicationSettingsCard({ commodityOptions }: { commodityOptions: { value: string }[] }) {
  const { t } = useTranslation();
  const licenseUrl = `${projectRepositoryUrl}/blob/main/LICENSE.md`;

  return (
    <Card
      className="settings-card app-info-card"
      title={<SectionTitle icon={<AppstoreOutlined />} label={t("settings.application")} />}
    >
      <div className="application-settings-card">
        <Form.Item
          label={<SectionTitle icon={null} label={t("settings.journalPath")} />}
          name="journalPath"
          rules={[{ required: true, message: t("settings.journalPathRequired") }]}
        >
          <PathInput
            placeholder={t("settings.journalPathPlaceholder")}
            pickerTitle={t("settings.pickJournalFile")}
          />
        </Form.Item>
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
        <div className="developer-settings">
          <div>
            <Typography.Text strong>{t("settings.developerOptions")}</Typography.Text>
            <Typography.Paragraph>{t("settings.advancedModeHelp")}</Typography.Paragraph>
          </div>
          <Form.Item name="powerUser" valuePropName="checked" noStyle>
            <Switch />
          </Form.Item>
        </div>
        <div className="application-meta-row">
          <Typography.Text>{t("settings.version", { version: packageJson.version })}</Typography.Text>
          <Space wrap>
            <Button href={projectRepositoryUrl} target="_blank">
              {t("settings.repository")}
            </Button>
            <Button href={licenseUrl} target="_blank">
              {t("settings.license", { license: packageJson.license })}
            </Button>
          </Space>
        </div>
      </div>
    </Card>
  );
}

export function SettingsRoute({
  form,
  initialValues,
  commodityOptions,
  hledgerStatus,
  onValuesChange,
}: {
  form: FormInstance<AppSettings>;
  initialValues: AppSettings;
  commodityOptions: { value: string }[];
  hledgerStatus: HledgerStatus | undefined;
  onValuesChange: (changed: Partial<AppSettings>, values: AppSettings) => void;
}) {
  const { t } = useTranslation();

  return (
    <Form<AppSettings>
      form={form}
      layout="vertical"
      initialValues={initialValues}
      onValuesChange={onValuesChange}
    >
      <Space direction="vertical" size={24} className="content-stack settings-stack">
        <ApplicationSettingsCard commodityOptions={commodityOptions} />

        <Card
          className="settings-card"
          title={<SectionTitle icon={<CodeOutlined />} label={t("settings.hledger")} />}
        >
          <Form.Item
            label={t("settings.hledgerExecutable")}
            name="hledgerPath"
          >
            <PathInput
              placeholder={hledgerStatus?.resolvedPath || t("settings.hledgerExecutablePlaceholder")}
              pickerTitle={t("settings.pickHledgerExecutable")}
              statusAddon={(
                <Tooltip
                  title={hledgerStatus?.source === "configured"
                    ? t("settings.hledgerUsingConfigured")
                    : hledgerStatus?.resolvedPath
                      ? t("settings.hledgerUsingDetected", { path: hledgerStatus.resolvedPath })
                      : t("settings.hledgerExecutableHelp")}
                >
                  <span>
                    {hledgerStatus?.source === "configured"
                      ? t("settings.configured")
                      : hledgerStatus?.resolvedPath
                        ? t("settings.detected")
                        : t("settings.fallback")}
                  </span>
                </Tooltip>
              )}
            />
          </Form.Item>
          <Descriptions column={2} size="small" className="settings-meta">
            <Descriptions.Item label={t("settings.status")}>
              {hledgerStatus?.available ? (
                <Tag color="success">{t("common.available")}</Tag>
              ) : (
                <Tag color="error">{t("common.unavailable")}</Tag>
              )}
            </Descriptions.Item>
            <Descriptions.Item label={t("hledger.version")}>
              {hledgerStatus?.version || t("common.notDetected")}
            </Descriptions.Item>
          </Descriptions>
        </Card>

      </Space>
    </Form>
  );
}
