import { useTranslation } from "react-i18next";
import type { AppView, NavigationItem } from "../types";
import styles from "./NavigationGroup.module.css";

export function NavigationGroup({
  items,
  activeKey,
  syncFooter,
  onSelect,
}: {
  items: NavigationItem[];
  activeKey: AppView;
  syncFooter?: {
    label: string;
    detail: string;
    tone: "neutral" | "success" | "warning" | "danger";
    onClick: () => void;
  };
  onSelect: (key: AppView) => void;
}) {
  const { t } = useTranslation();

  return (
    <div className={styles.nav_group}>
      <div className={styles.nav_items}>
        {items.map((item) => (
          <button
            key={item.key}
            type="button"
            className={`${styles.nav_item} ${activeKey === item.key ? styles.is_active : ""} ${item.disabled ? styles.is_disabled : ""}`}
            disabled={item.disabled}
            onClick={() => {
              if (!item.disabled) onSelect(item.key);
            }}
          >
            {item.icon}
            <span>{t(item.label)}</span>
            {item.badge ? <span className={styles.nav_badge}>{item.badge}</span> : null}
            {item.shortcut ? <span className={styles.nav_shortcut}>{item.shortcut}</span> : null}
            {item.disabled ? <span className={styles.nav_item_lock}>🔒</span> : null}
          </button>
        ))}
      </div>
      {syncFooter ? (
        <button type="button" className={styles.sync_footer} onClick={syncFooter.onClick}>
          <span className={`${styles.sync_dot} ${styles[`sync_dot_${syncFooter.tone}`]}`} />
          <span>
            <span className={styles.sync_label}>{syncFooter.label}</span>
            <span className={styles.sync_detail}>{syncFooter.detail}</span>
          </span>
        </button>
      ) : null}
    </div>
  );
}
