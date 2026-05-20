import { FileTextOutlined, SearchOutlined } from "@ant-design/icons";
import { Input, Typography } from "antd";
import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import type { JournalTransaction } from "../types";
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
    queryFn: () => invoke<JournalTransaction[]>("search_journal", { query: debouncedQuery, limit: 12 }),
    enabled: open && debouncedQuery.trim().length > 0,
    retry: false,
    placeholderData: (previous) => previous,
  });

  const results = useMemo(() => resultsQuery.data ?? [], [resultsQuery.data]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [debouncedQuery]);

  if (!open) return null;

  function execute(result: JournalTransaction | undefined) {
    if (!result) return;
    onTransaction(result);
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
        <div className={styles.inputWrap}>
          <Input
            autoFocus
            size="large"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            prefix={<SearchOutlined style={{ opacity: 0.5 }} />}
            placeholder="Search the journal…"
          />
        </div>
        <div className={styles.results}>
          {debouncedQuery.trim().length === 0 ? (
            <Typography.Text type="secondary" className={styles.empty}>
              Start typing to search transactions
            </Typography.Text>
          ) : resultsQuery.isFetching && results.length === 0 ? (
            <Typography.Text type="secondary" className={styles.empty}>
              Searching…
            </Typography.Text>
          ) : results.length === 0 ? (
            <Typography.Text type="secondary" className={styles.empty}>
              No matching transactions
            </Typography.Text>
          ) : (
            results.map((result, index) => (
              <button
                key={result.id}
                type="button"
                className={`${styles.result} ${index === selectedIndex ? styles.resultSelected : ""}`}
                onMouseEnter={() => setSelectedIndex(index)}
                onClick={() => execute(result)}
              >
                <span className={styles.icon}><FileTextOutlined /></span>
                <span>
                  <span className={styles.primary}>{result.description || result.display.formatted}</span>
                  <span className={styles.secondary}>{resultSubtitle(result)}</span>
                </span>
              </button>
            ))
          )}
        </div>
        <div className={styles.help}>
          <span>↑↓ Navigate</span>
          <span>Enter Open</span>
          <span>Esc Close</span>
        </div>
      </div>
    </div>
  );
}

function resultSubtitle(tx: JournalTransaction): string {
  const flow = [...tx.display.flow.from, ...tx.display.flow.to].join(" → ");
  return [tx.date, tx.display.formatted, flow].filter(Boolean).join(" · ");
}
