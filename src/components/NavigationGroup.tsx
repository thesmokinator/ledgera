import { useTranslation } from "react-i18next";
import type { NavigationItem } from "../types";

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
    <div className="nav-group">
      {items.map((item) => (
        <button
          key={item.key}
          type="button"
          className={`nav-item ${activeKey === item.key ? "is-active" : ""} ${item.disabled ? "is-disabled" : ""}`}
          disabled={item.disabled}
          onClick={() => {
            if (!item.disabled) onSelect(item.key);
          }}
        >
          {item.icon}
          <span>{t(item.label)}</span>
          {item.disabled ? <span className="nav-item-lock">🔒</span> : null}
        </button>
      ))}
    </div>
  );
}
