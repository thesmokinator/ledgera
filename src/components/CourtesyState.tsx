import { Card, Typography } from "antd";
import { FolderOpenOutlined } from "@ant-design/icons";
import { useTranslation } from "react-i18next";

export function CourtesyState({
  reasons,
  details,
}: {
  reasons: string[];
  details?: string;
}) {
  const { t } = useTranslation();

  return (
    <Card className="courtesy-card">
      <div className="courtesy-icon">
        <FolderOpenOutlined />
      </div>
      <Typography.Title level={4} className="courtesy-title">
        {t("settings.configureJournalTitle")}
      </Typography.Title>
      <Typography.Text type="secondary" className="courtesy-description">
        {t("settings.configureJournalDescription")}
      </Typography.Text>
      {reasons.length > 0 ? (
        <div className="courtesy-reasons">
          {reasons.map((reason) => (
            <div key={reason} className="courtesy-reason-item">{reason}</div>
          ))}
        </div>
      ) : null}
      {details ? (
        <pre className="courtesy-details">{details}</pre>
      ) : null}
    </Card>
  );
}
