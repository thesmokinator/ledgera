import { FileTextOutlined, FolderOutlined, PlusOutlined, SearchOutlined, SettingOutlined } from "@ant-design/icons";
import { Input, Typography } from "antd";
import { useEffect, useMemo, useState } from "react";
import type { CommandPaletteCommand, CommandPaletteResult } from "../utils/search";
import { searchCommandPalette } from "../utils/search";
import type { JournalTransaction } from "../types";
import styles from "./CommandPalette.module.css";

export function CommandPalette({
  open,
  commands,
  accounts,
  transactions,
  onClose,
  onCommand,
  onAccount,
  onTransaction,
}: {
  open: boolean;
  commands: CommandPaletteCommand[];
  accounts: string[];
  transactions: JournalTransaction[];
  onClose: () => void;
  onCommand: (commandId: string) => void;
  onAccount: (account: string) => void;
  onTransaction: (transaction: JournalTransaction) => void;
}) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);

  const results = useMemo(
    () => searchCommandPalette({ query, commands, accounts, transactions, limit: 12 }),
    [query, commands, accounts, transactions],
  );

  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
    }
  }, [open]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  if (!open) return null;

  function execute(result: CommandPaletteResult | undefined) {
    if (!result) return;
    if (result.type === "command") onCommand(result.id);
    if (result.type === "account") onAccount(result.account);
    if (result.type === "transaction") onTransaction(result.transaction);
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

  const grouped = groupResults(results);

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
            placeholder="Search accounts, transactions, commands…"
          />
        </div>
        <div className={styles.results}>
          {results.length === 0 ? (
            <Typography.Text type="secondary" className={styles.empty}>
              No results
            </Typography.Text>
          ) : grouped.map(({ title, items }) => (
            <div key={title}>
              <div className={styles.groupTitle}>{title}</div>
              {items.map((result) => {
                const index = results.indexOf(result);
                return (
                  <button
                    key={`${result.type}:${result.id}`}
                    type="button"
                    className={`${styles.result} ${index === selectedIndex ? styles.resultSelected : ""}`}
                    onMouseEnter={() => setSelectedIndex(index)}
                    onClick={() => execute(result)}
                  >
                    <span className={styles.icon}>{resultIcon(result)}</span>
                    <span>
                      <span className={styles.primary}>{resultTitle(result)}</span>
                      <span className={styles.secondary}>{resultSubtitle(result)}</span>
                    </span>
                    {result.type === "command" && result.shortcut ? (
                      <span className={styles.shortcut}>{result.shortcut}</span>
                    ) : null}
                  </button>
                );
              })}
            </div>
          ))}
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

function groupResults(results: CommandPaletteResult[]): { title: string; items: CommandPaletteResult[] }[] {
  const groups = [
    { title: "Commands", type: "command" as const },
    { title: "Accounts", type: "account" as const },
    { title: "Transactions", type: "transaction" as const },
  ];

  return groups
    .map((group) => ({
      title: group.title,
      items: results.filter((result) => result.type === group.type),
    }))
    .filter((group) => group.items.length > 0);
}

function resultIcon(result: CommandPaletteResult) {
  if (result.type === "command") {
    return result.id === "new-transaction" ? <PlusOutlined /> : <SettingOutlined />;
  }
  if (result.type === "account") return <FolderOutlined />;
  return <FileTextOutlined />;
}

function resultTitle(result: CommandPaletteResult): string {
  if (result.type === "command") return result.label;
  if (result.type === "account") return result.account;
  return result.transaction.description || result.transaction.display.formatted;
}

function resultSubtitle(result: CommandPaletteResult): string {
  if (result.type === "command") return "Command";
  if (result.type === "account") return "Account";
  const tx = result.transaction;
  const flow = [...tx.display.flow.from, ...tx.display.flow.to].join(" → ");
  return [tx.date, tx.display.formatted, flow].filter(Boolean).join(" · ");
}
