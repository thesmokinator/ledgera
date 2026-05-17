import { Button, Card, Typography } from "antd";
import { SettingOutlined, WarningOutlined } from "@ant-design/icons";
import { useTranslation } from "react-i18next";

export function CourtesyState({
  reasons,
  details,
  onConfigure,
}: {
  reasons: string[];
  details?: string;
  onConfigure: () => void;
}) {
  const { t } = useTranslation();

  return (
    <Card className="courtesy-card">
      <div className="courtesy-icon">
        <WarningOutlined />
      </div>
      <Typography.Title level={3}>{t("settings.configureJournalTitle")}</Typography.Title>
      <Typography.Text>{t("settings.configureJournalDescription")}</Typography.Text>
      <ul className="courtesy-reasons">
        {reasons.map((reason) => (
          <li key={reason}>{reason}</li>
        ))}
      </ul>
      {details ? <pre className="courtesy-details">{details}</pre> : null}
      <Button type="primary" icon={<SettingOutlined />} onClick={onConfigure}>
        {t("common.configure")}
      </Button>
    </Card>
  );
}
