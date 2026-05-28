import type { AmountTint } from "../types";
import styles from "./Amount.module.css";

type AmountKind = "expense" | "income" | "transfer" | "investment" | "unknown" | string;

export function classSuffix(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function amountClass({ tint, kind }: { tint: AmountTint; kind?: AmountKind }): string {
  return styles[`amount_kind_${kind}`] ?? styles[`amount_${tint}`] ?? "";
}

export function Amount({
  formatted,
  tint = "neutral",
  kind,
  className,
}: {
  formatted: string;
  tint?: AmountTint;
  kind?: AmountKind;
  className?: string;
}) {
  return (
    <span
      className={`${styles.amount} ${amountClass({ tint, kind })}${className ? ` ${className}` : ""}`}
    >
      {formatted}
    </span>
  );
}
