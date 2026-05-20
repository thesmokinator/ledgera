import { useTranslation } from "react-i18next";
import type { NavigationItem } from "../types";
import styles from "./NavigationGroup.module.css";

export function NavigationGroup({
  items,
  activeKey,
  onSelect,
}: {
  items: NavigationItem[];
  activeKey: string;
  onSelect: (key: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <div className={styles.nav_group}>
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
          {item.shortcut ? <span className={styles.nav_shortcut}>{item.shortcut}</span> : null}
          {item.disabled ? <span className={styles.nav_item_lock}>🔒</span> : null}
        </button>
      ))}
    </div>
  );
}
