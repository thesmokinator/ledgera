import { Card, Typography } from "antd";
import { FolderOpenOutlined } from "@ant-design/icons";
import { useTranslation } from "react-i18next";
import styles from "./CourtesyState.module.css";

export function CourtesyState({
  reasons,
  details,
}: {
  reasons: string[];
  details?: string;
}) {
  const { t } = useTranslation();

  return (
    <Card className={styles.card}>
      <div className={styles.icon}>
        <FolderOpenOutlined />
      </div>
      <Typography.Title level={4} className={styles.title}>
        {t("settings.configure_journal_title")}
      </Typography.Title>
      <Typography.Text type="secondary" className={styles.description}>
        {t("settings.configure_journal_description")}
      </Typography.Text>
      {reasons.length > 0 ? (
        <div className={styles.reasons}>
          {reasons.map((reason) => (
            <div key={reason} className={styles.reason_item}>{reason}</div>
          ))}
        </div>
      ) : null}
      {details ? (
        <pre className={styles.details}>{details}</pre>
      ) : null}
    </Card>
  );
}
