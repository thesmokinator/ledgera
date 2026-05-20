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
    <div className={styles.navGroup}>
      {items.map((item) => (
        <button
          key={item.key}
          type="button"
          className={`${styles.navItem} ${activeKey === item.key ? styles.isActive : ""} ${item.disabled ? styles.isDisabled : ""}`}
          disabled={item.disabled}
          onClick={() => {
            if (!item.disabled) onSelect(item.key);
          }}
        >
          {item.icon}
          <span>{t(item.label)}</span>
          {item.shortcut ? <span className={styles.navShortcut}>{item.shortcut}</span> : null}
          {item.disabled ? <span className={styles.navItemLock}>🔒</span> : null}
        </button>
      ))}
    </div>
  );
}
