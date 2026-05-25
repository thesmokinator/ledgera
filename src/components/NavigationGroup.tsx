import { useTranslation } from "react-i18next";
import type { AppView, NavigationItem } from "../types";
import styles from "./NavigationGroup.module.css";

export function NavigationGroup({
  items,
  activeKey,
  onSelect,
}: {
  items: NavigationItem[];
  activeKey: AppView;
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
            {item.badge ? (
              <span className={`${styles.nav_badge} ${item.badgeTone === "danger" ? styles.nav_badge_danger : ""}`}>
                {item.badge}
              </span>
            ) : null}
            {item.shortcut ? <span className={styles.nav_shortcut}>{item.shortcut}</span> : null}
            {item.disabled ? <span className={styles.nav_item_lock}>🔒</span> : null}
          </button>
        ))}
      </div>
    </div>
  );
}
