import type { AmountTint } from "../types";
import styles from "./Amount.module.css";

export function Amount({
  formatted,
  tint = "neutral",
  className,
}: {
  formatted: string;
  tint?: AmountTint;
  className?: string;
}) {
  return (
    <span
      className={`${styles.amount} ${styles[`amount${tint.charAt(0).toUpperCase() + tint.slice(1)}`]}${className ? ` ${className}` : ""}`}
    >
      {formatted}
    </span>
  );
}
