import { FileTextOutlined, SearchOutlined } from "@ant-design/icons";
import { Input, Typography } from "antd";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import type { JournalSearchMatch, JournalSearchResult, JournalTransaction, SearchMatchRange } from "../types";
import styles from "./CommandPalette.module.css";

export function CommandPalette({
  open,
  onClose,
  onTransaction,
}: {
  open: boolean;
  onClose: () => void;
  onTransaction: (transaction: JournalTransaction) => void;
}) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [debouncedQuery, setDebouncedQuery] = useState("");

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setDebouncedQuery("");
    setSelectedIndex(0);
  }, [open]);

  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedQuery(query), 180);
    return () => window.clearTimeout(timer);
  }, [query]);

  const resultsQuery = useQuery({
    queryKey: ["journal-search", debouncedQuery],
    queryFn: () => invoke<JournalSearchResult[]>("search_journal", { query: debouncedQuery, limit: 12 }),
    enabled: open && debouncedQuery.trim().length > 0,
    retry: false,
    placeholderData: (previous) => previous,
  });

  const results = useMemo(() => resultsQuery.data ?? [], [resultsQuery.data]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [debouncedQuery]);

  if (!open) return null;

  function execute(result: JournalSearchResult | undefined) {
    if (!result) return;
    onTransaction(result.transaction);
    onClose();
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelectedIndex((index) => Math.min(index + 1, results.length - 1));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelectedIndex((index) => Math.max(index - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      execute(results[selectedIndex]);
    }
  }

  return (
    <div className={styles.overlay} onMouseDown={onClose}>
      <div className={styles.panel} onMouseDown={(event) => event.stopPropagation()}>
        <div className={styles.input_wrap}>
          <Input
            autoFocus
            size="large"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            prefix={<SearchOutlined style={{ opacity: 0.5 }} />}
            placeholder={t("search.placeholder")}
          />
        </div>
        <div className={styles.results}>
          {debouncedQuery.trim().length === 0 ? (
            <Typography.Text type="secondary" className={styles.empty}>
              {t("search.start_typing")}
            </Typography.Text>
          ) : resultsQuery.isFetching && results.length === 0 ? (
            <Typography.Text type="secondary" className={styles.empty}>
              {t("search.searching")}
            </Typography.Text>
          ) : results.length === 0 ? (
            <Typography.Text type="secondary" className={styles.empty}>
              {t("search.no_results")}
            </Typography.Text>
          ) : (
            results.map((result, index) => (
              <button
                key={result.transaction.id}
                type="button"
                className={`${styles.result} ${index === selectedIndex ? styles.result_selected : ""}`}
                onMouseEnter={() => setSelectedIndex(index)}
                onClick={() => execute(result)}
              >
                <span className={styles.icon}><FileTextOutlined /></span>
                <span>
                  <span className={styles.primary}>
                    {renderHighlightedTitle(result)}
                  </span>
                  <span className={styles.secondary}>{resultSubtitle(result.transaction)}</span>
                  {bestSecondaryMatch(result.matches) ? (
                    <span className={styles.match_context}>
                      {matchLabel(bestSecondaryMatch(result.matches)!, t)}: {renderHighlightedText(bestSecondaryMatch(result.matches)!.value, bestSecondaryMatch(result.matches)!.ranges)}
                    </span>
                  ) : null}
                </span>
              </button>
            ))
          )}
        </div>
        <div className={styles.help}>
          <span>{t("search.navigate_shortcut")}</span>
          <span>{t("search.open_shortcut")}</span>
          <span>{t("search.close_shortcut")}</span>
        </div>
      </div>
    </div>
  );
}

function renderHighlightedTitle(result: JournalSearchResult): ReactNode {
  const tx = result.transaction;
  const title = tx.description || tx.display.formatted;
  const descriptionMatch = result.matches.find((match) => match.field === "description");
  return renderHighlightedText(title, descriptionMatch?.ranges ?? []);
}

function bestSecondaryMatch(matches: JournalSearchMatch[]): JournalSearchMatch | undefined {
  return matches.find((match) => match.field === "comment")
    ?? matches.find((match) => match.field === "account");
}

function matchLabel(match: JournalSearchMatch, t: (key: string) => string): string {
  if (match.field === "comment") return t("transactions.comment");
  if (match.field === "account") return t("transactions.account");
  return t("transactions.description");
}

function renderHighlightedText(value: string, ranges: SearchMatchRange[]): ReactNode {
  if (ranges.length === 0) return value;

  const characters = Array.from(value);
  const parts: ReactNode[] = [];
  let cursor = 0;

  for (const range of ranges) {
    const start = Math.max(0, Math.min(range.start, characters.length));
    const end = Math.max(start, Math.min(range.end, characters.length));
    if (start > cursor) {
      parts.push(characters.slice(cursor, start).join(""));
    }
    if (end > start) {
      parts.push(
        <mark className={styles.highlight} key={`${start}:${end}`}>
          {characters.slice(start, end).join("")}
        </mark>,
      );
    }
    cursor = end;
  }

  if (cursor < characters.length) {
    parts.push(characters.slice(cursor).join(""));
  }

  return parts;
}

function resultSubtitle(tx: JournalTransaction): string {
  const flow = [...tx.display.flow.from, ...tx.display.flow.to].join(" → ");
  return [tx.date, tx.display.formatted, flow].filter(Boolean).join(" · ");
}
