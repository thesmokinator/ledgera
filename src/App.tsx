import {
  AutoComplete,
  Button,
  Card,
  ConfigProvider,
  DatePicker,
  Descriptions,
  Form,
  Input,
  Layout,
  Modal,
  Select,
  Space,
  Switch,
  Table,
  Tooltip,
  Tabs,
  Tag,
  Typography,
  message,
  theme,
} from "antd";
import {
  AppstoreOutlined,
  BankOutlined,
  CodeOutlined,
  DeleteOutlined,
  EditOutlined,
  FileTextOutlined,
  HomeOutlined,
  LeftOutlined,
  PlusOutlined,
  RightOutlined,
  ReloadOutlined,
  SettingOutlined,
  UploadOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import dayjs from "dayjs";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import packageJson from "../package.json";
import "./App.css";

type ThemePreference = "system" | "dark" | "light";

type AppSettings = {
  journalPath: string;
  hledgerPath: string;
  theme: ThemePreference;
  powerUser: boolean;
};

type HledgerStatus = {
  available: boolean;
  version: string;
  message: string;
  resolvedPath: string;
  source: "configured" | "detected" | "fallback";
};

type JournalPosting = {
  account: string;
  amount: string;
  commodity: string;
  comment: string;
  raw: string;
};

type TransactionDisplay = {
  account: string;
  amount: string;
  kind: string;
};

type JournalTransaction = {
  id: string;
  sourceFile: string;
  date: string;
  status: string;
  code: string;
  description: string;
  postings: JournalPosting[];
  display: TransactionDisplay;
  raw: string;
  startLine: number;
  endLine: number;
};

type DashboardSummary = {
  monthlyTransactions: JournalTransaction[];
  scheduledTransactions: JournalTransaction[];
  activeAccountsCount: number;
};

type JournalSummary = {
  path: string;
  transactions: JournalTransaction[];
  commodities: string[];
  dashboard: DashboardSummary;
};

type AutocompleteSuggestions = {
  codes: string[];
  descriptions: string[];
  accounts: string[];
  commodities: string[];
};

type PostingInput = {
  account: string;
  amount: string;
  commodity: string;
  comment: string;
};

type TransactionInput = {
  date: string;
  status: string;
  code: string;
  description: string;
  postings: PostingInput[];
};

type NavigationItem = {
  key: string;
  label: string;
  icon: ReactNode;
};

type AccountActivityRange = "current-month" | "30" | "60" | "90" | "180" | "365";

type AccountSummary = {
  account: string;
  transactions: number;
  accountTransactions: JournalTransaction[];
};




const projectName = packageJson.name;
const projectRepositoryUrl = packageJson.repository.url.replace(/\.git$/, "");
const journalDateFormat = "YYYY-MM-DD";

function todayJournalDate(): string {
  return dayjs().format(journalDateFormat);
}

const emptyTransaction: TransactionInput = {
  date: todayJournalDate(),
  status: "",
  code: "",
  description: "",
  postings: [
    { account: "", amount: "", commodity: "", comment: "" },
    { account: "", amount: "", commodity: "", comment: "" },
  ],
};

const defaultSettings: AppSettings = {
  journalPath: "",
  hledgerPath: "",
  theme: "system",
  powerUser: false,
};

const primaryNavigation: NavigationItem[] = [
  { key: "transactions", label: "common.transactions", icon: <HomeOutlined /> },
  { key: "accounts", label: "common.accounts", icon: <BankOutlined /> },
  { key: "settings", label: "common.settings", icon: <SettingOutlined /> },
];

const accountActivityRangeOptions: AccountActivityRange[] = [
  "current-month",
  "30",
  "60",
  "90",
  "180",
  "365",
];

function toAutocompleteOptions(values: string[]) {
  return values.map((value) => ({ value }));
}



function normalizeSettings(settings?: Partial<AppSettings>): AppSettings {
  return {
    ...defaultSettings,
    ...settings,
    theme: settings?.theme ?? "system",
    powerUser: settings?.powerUser ?? false,
  };
}

/** Invokes a typed Tauri command. */
function callCommand<TResponse, TPayload extends Record<string, unknown> = Record<string, never>>(
  command: string,
  payload?: TPayload,
): Promise<TResponse> {
  return invoke<TResponse>(command, payload);
}

/** Converts a transaction returned by Rust into editable form values. */
function toTransactionInput(transaction: JournalTransaction): TransactionInput {
  return {
    date: transaction.date,
    status: transaction.status,
    code: transaction.code,
    description: transaction.description,
    postings:
      transaction.postings.length > 0
        ? transaction.postings.map((posting) => ({
          account: posting.account,
          amount: posting.amount,
          commodity: posting.commodity,
          comment: posting.comment,
        }))
        : emptyTransaction.postings,
  };
}

/** Formats compact dashboard counters. */
function isValidJournalDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return false;
  }

  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));
  return (
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
}

function formatCount(value: number): string {
  return new Intl.NumberFormat("en", { maximumFractionDigits: 0 }).format(value);
}

function isSameJournalMonth(date: string, month: dayjs.Dayjs): boolean {
  const parsedDate = dayjs(date, journalDateFormat, true);
  return parsedDate.isValid() && parsedDate.isSame(month, "month");
}

function isExecutedTransaction(transaction: JournalTransaction): boolean {
  const parsedDate = dayjs(transaction.date, journalDateFormat, true);
  return parsedDate.isValid() && !parsedDate.isAfter(dayjs(), "day");
}

function transactionIncludesAccount(transaction: JournalTransaction, account: string): boolean {
  return transaction.postings.some(
    (posting) => posting.account.trim().toLowerCase() === account.toLowerCase(),
  );
}

function isInAccountActivityRange(transaction: JournalTransaction, range: AccountActivityRange): boolean {
  const parsedDate = dayjs(transaction.date, journalDateFormat, true);
  if (!parsedDate.isValid()) {
    return false;
  }

  const today = dayjs().startOf("day");
  if (range === "current-month") {
    return parsedDate.isSame(today, "month");
  }

  const days = Number(range);
  const rangeStart = today.subtract(days - 1, "day");
  return !parsedDate.isBefore(rangeStart, "day") && !parsedDate.isAfter(today, "day");
}

function collectAccounts(
  transactions: JournalTransaction[],
  visibleTransactions: JournalTransaction[],
): AccountSummary[] {
  const accountNames = new Map<string, string>();

  transactions.forEach((transaction) => {
    transaction.postings.forEach((posting) => {
      const account = posting.account.trim();
      if (account) {
        accountNames.set(account.toLowerCase(), account);
      }
    });
  });

  return Array.from(accountNames.values())
    .map((account) => {
      const accountTransactions = visibleTransactions.filter((transaction) =>
        transactionIncludesAccount(transaction, account),
      );

      return {
        account,
        transactions: accountTransactions.length,
        accountTransactions,
      };
    })
    .sort((left, right) => left.account.localeCompare(right.account));
}



function useSystemTheme() {
  const [isDark, setIsDark] = useState(() =>
    window.matchMedia("(prefers-color-scheme: dark)").matches,
  );

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = (event: MediaQueryListEvent) => setIsDark(event.matches);
    mediaQuery.addEventListener("change", onChange);
    return () => mediaQuery.removeEventListener("change", onChange);
  }, []);

  return isDark;
}



/** Renders one sidebar navigation block. */
function NavigationGroup({ items, activeKey, onSelect }: {
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
          className={`nav-item ${activeKey === item.key ? "is-active" : ""}`}
          onClick={() => onSelect(item.key)}
        >
          {item.icon}
          <span>{t(item.label)}</span>
        </button>
      ))}
    </div>
  );
}

function CourtesyState({
  reasons,
  details,
  onConfigure,
}: {
  reasons: string[];
  details?: string;
  onConfigure: () => void;
}) {
  const { t } = useTranslation();

  return (
    <Card className="courtesy-card">
      <div className="courtesy-icon">
        <WarningOutlined />
      </div>
      <Typography.Title level={3}>{t("settings.configureJournalTitle")}</Typography.Title>
      <Typography.Text>{t("settings.configureJournalDescription")}</Typography.Text>
      <ul className="courtesy-reasons">
        {reasons.map((reason) => (
          <li key={reason}>{reason}</li>
        ))}
      </ul>
      {details ? <pre className="courtesy-details">{details}</pre> : null}
      <Button type="primary" icon={<SettingOutlined />} onClick={onConfigure}>
        {t("common.configure")}
      </Button>
    </Card>
  );
}

function TransactionsTable({
  transactions,
  loading,
  powerUser,
  onEdit,
  onDelete,
}: {
  transactions: JournalTransaction[];
  loading: boolean;
  powerUser: boolean;
  onEdit: (transaction: JournalTransaction) => void;
  onDelete: (id: string) => void;
}) {
  const { t } = useTranslation();
  const [modal, modalContextHolder] = Modal.useModal();

  function confirmDelete(transaction: JournalTransaction) {
    modal.confirm({
      title: t("transactions.deleteTransactionAction"),
      content: t("transactions.deleteTransactionDescription"),
      okText: t("transactions.delete"),
      cancelText: t("common.cancel"),
      okButtonProps: { danger: true },
      centered: true,
      onOk: () => onDelete(transaction.id),
    });
  }

  return (
    <>
      {modalContextHolder}
      <Table<JournalTransaction>
        rowKey="id"
        loading={loading}
        dataSource={transactions}
        pagination={{ pageSize: 8 }}
        expandable={
          powerUser
            ? {
              expandedRowRender: (transaction) => (
                <pre className="transaction-raw">{transaction.raw}</pre>
              ),
            }
            : undefined
        }
        columns={[
          { title: t("transactions.date"), dataIndex: "date", width: 132 },
          { title: t("transactions.status"), dataIndex: "status", width: 88, render: (status: string) => status || "-" },
          { title: t("transactions.description"), dataIndex: "description" },
          {
            title: t("transactions.account"),
            width: 260,
            render: (_, transaction) => transaction.display.account,
          },
          {
            title: t("transactions.amount"),
            width: 160,
            align: "right",
            render: (_, transaction) => (
              <span className={`transaction-amount transaction-amount-${transaction.display.kind}`}>
                {transaction.display.amount}
              </span>
            ),
          },
          ...(powerUser
            ? [
              {
                title: t("transactions.lines"),
                width: 120,
                render: (_: unknown, transaction: JournalTransaction) =>
                  `${transaction.startLine}-${transaction.endLine}`,
              },
            ]
            : []),
          {
            title: t("transactions.actions"),
            width: 136,
            render: (_, transaction) => (
              <Space>
                <Button aria-label={t("transactions.editTransactionAction")} icon={<EditOutlined />} onClick={() => onEdit(transaction)} />
                <Button
                  danger
                  aria-label={t("transactions.deleteTransactionAction")}
                  icon={<DeleteOutlined />}
                  onClick={() => confirmDelete(transaction)}
                />
              </Space>
            ),
          },
        ]}
      />
    </>
  );
}

function SectionTitle({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <Space className="settings-section-title">
      {icon}
      <span>{label}</span>
    </Space>
  );
}

function PathInput({
  value,
  onChange,
  placeholder,
  pickerTitle,
  statusAddon,
}: {
  value?: string;
  onChange?: (value: string) => void;
  placeholder: string;
  pickerTitle: string;
  statusAddon?: ReactNode;
}) {
  const inputRef = useRef<HTMLInputElement>(null);

  function selectFile(file?: File) {
    if (!file) {
      return;
    }

    onChange?.(((file as File & { path?: string }).path || file.name));
  }

  return (
    <>
      <Space.Compact block className="path-input-group">
        <Input value={value} onChange={(event) => onChange?.(event.target.value)} placeholder={placeholder} />
        {statusAddon ? <div className="path-input-addon">{statusAddon}</div> : null}
        <Tooltip title={pickerTitle}>
          <Button icon={<UploadOutlined />} onClick={() => inputRef.current?.click()} />
        </Tooltip>
      </Space.Compact>
      <input
        ref={inputRef}
        className="hidden-file-input"
        type="file"
        tabIndex={-1}
        aria-hidden="true"
        style={{ display: "none" }}
        onChange={(event) => selectFile(event.target.files?.[0])}
      />
    </>
  );
}

function ApplicationSettingsCard() {
  const { t } = useTranslation();
  const licenseUrl = `${projectRepositoryUrl}/blob/main/LICENSE.md`;

  return (
    <Card
      className="settings-card app-info-card"
      title={<SectionTitle icon={<AppstoreOutlined />} label={t("settings.application")} />}
    >
      <div className="application-settings-card">
        <Form.Item
          label={<SectionTitle icon={<FileTextOutlined />} label={t("settings.journalPath")} />}
          name="journalPath"
          rules={[{ required: true, message: t("settings.journalPathRequired") }]}
        >
          <PathInput
            placeholder={t("settings.journalPathPlaceholder")}
            pickerTitle={t("settings.pickJournalFile")}
          />
        </Form.Item>
        <Form.Item label={t("settings.theme")} name="theme" rules={[{ required: true }]}>
          <Select
            options={[
              { value: "system", label: t("settings.themeSystem") },
              { value: "dark", label: t("settings.themeDark") },
              { value: "light", label: t("settings.themeLight") },
            ]}
          />
        </Form.Item>
        <div className="developer-settings">
          <div>
            <Typography.Text strong>{t("settings.developerOptions")}</Typography.Text>
            <Typography.Paragraph>{t("settings.advancedModeHelp")}</Typography.Paragraph>
          </div>
          <Form.Item name="powerUser" valuePropName="checked" noStyle>
            <Switch />
          </Form.Item>
        </div>
        <div className="application-meta-row">
          <Typography.Text>{t("settings.version", { version: packageJson.version })}</Typography.Text>
          <Space wrap>
            <Button href={projectRepositoryUrl} target="_blank">
              {t("settings.repository")}
            </Button>
            <Button href={licenseUrl} target="_blank">
              {t("settings.license", { license: packageJson.license })}
            </Button>
          </Space>
        </div>
      </div>
    </Card>
  );
}

/** Renders the Ledgera desktop application. */
function App() {
  const [activeView, setActiveView] = useState("transactions");
  const [activeMonth, setActiveMonth] = useState(() => dayjs().startOf("month"));
  const [accountActivityRange, setAccountActivityRange] = useState<AccountActivityRange>("current-month");
  const [editingTransaction, setEditingTransaction] = useState<JournalTransaction | null>(null);
  const [isTransactionModalOpen, setTransactionModalOpen] = useState(false);
  const [transactionForm] = Form.useForm<TransactionInput>();
  const [settingsForm] = Form.useForm<AppSettings>();
  const queryClient = useQueryClient();
  const [messageApi, contextHolder] = message.useMessage();
  const { t } = useTranslation();
  const systemPrefersDark = useSystemTheme();

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: async () => normalizeSettings(await callCommand<AppSettings>("get_app_settings")),
  });

  const activeSettings = normalizeSettings(settingsQuery.data);
  const isDarkTheme = activeSettings.theme === "system" ? systemPrefersDark : activeSettings.theme === "dark";
  const activeTitle = activeView === "settings" ? t("common.settings") : t(`common.${activeView}`);

  const hledgerQuery = useQuery({
    queryKey: ["hledger-status"],
    queryFn: () => callCommand<HledgerStatus>("check_hledger"),
  });

  const transactionsQuery = useQuery({
    queryKey: ["transactions"],
    queryFn: () => callCommand<JournalSummary>("list_transactions"),
    enabled: Boolean(settingsQuery.data?.journalPath),
    retry: false,
  });

  const autocompleteQuery = useQuery({
    queryKey: ["autocomplete-suggestions"],
    queryFn: () => callCommand<AutocompleteSuggestions>("get_autocomplete_suggestions"),
    enabled: Boolean(settingsQuery.data?.journalPath),
    retry: false,
  });

  const updateSettingsMutation = useMutation({
    mutationFn: (settings: AppSettings) =>
      callCommand<AppSettings, { settings: AppSettings }>("update_app_settings", {
        settings: normalizeSettings(settings),
      }),
    onSuccess: async (settings) => {
      queryClient.setQueryData(["settings"], normalizeSettings(settings));
      await queryClient.invalidateQueries({ queryKey: ["hledger-status"] });
      await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
    },
    onError: (error) => messageApi.error(String(error)),
  });

  const createTransactionMutation = useMutation({
    mutationFn: (input: TransactionInput) =>
      callCommand<JournalSummary, { input: TransactionInput }>("create_transaction", { input }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transactionCreated"));
      setTransactionModalOpen(false);
      await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
    },
    onError: (error) => messageApi.error(String(error)),
  });

  const updateTransactionMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: TransactionInput }) =>
      callCommand<JournalSummary, { id: string; input: TransactionInput }>("update_transaction", {
        id,
        input,
      }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transactionUpdated"));
      setTransactionModalOpen(false);
      setEditingTransaction(null);
      await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
    },
    onError: (error) => messageApi.error(String(error)),
  });

  const deleteTransactionMutation = useMutation({
    mutationFn: (id: string) =>
      callCommand<JournalSummary, { id: string }>("delete_transaction", { id }),
    onSuccess: async () => {
      messageApi.success(t("transactions.transactionDeleted"));
      await queryClient.invalidateQueries({ queryKey: ["transactions"] });
      await queryClient.invalidateQueries({ queryKey: ["autocomplete-suggestions"] });
    },
    onError: (error) => messageApi.error(String(error)),
  });

  const autocompleteSuggestions = autocompleteQuery.data ?? {
    codes: [],
    descriptions: [],
    accounts: [],
    commodities: [],
  };
  const codeOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.codes),
    [autocompleteSuggestions.codes],
  );
  const descriptionOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.descriptions),
    [autocompleteSuggestions.descriptions],
  );
  const accountOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.accounts),
    [autocompleteSuggestions.accounts],
  );
  const commodityOptions = useMemo(
    () => toAutocompleteOptions(autocompleteSuggestions.commodities),
    [autocompleteSuggestions.commodities],
  );
  const defaultCommodity = autocompleteSuggestions.commodities[0] ?? "";
  const hasConfiguredJournal = activeSettings.journalPath.trim().length > 0;
  const hledgerUnavailable = hledgerQuery.isFetched && hledgerQuery.data?.available === false;
  const journalLoadError = transactionsQuery.isError ? String(transactionsQuery.error) : "";
  const courtesyReasons = [
    !hasConfiguredJournal ? t("settings.noJournalConfigured") : null,
    journalLoadError ? t("settings.journalReadFailed") : null,
    hledgerUnavailable ? t("settings.hledgerNotFound") : null,
  ].filter((reason): reason is string => Boolean(reason));
  const shouldShowCourtesy = courtesyReasons.length > 0;


  const transactions = transactionsQuery.data?.transactions ?? [];
  const visibleMonthTransactions = transactions.filter((transaction) =>
    isSameJournalMonth(transaction.date, activeMonth),
  );
  const monthlyTransactions = visibleMonthTransactions.filter(isExecutedTransaction);
  const scheduledTransactions = visibleMonthTransactions.filter(
    (transaction) => !isExecutedTransaction(transaction),
  );
  const visibleAccountTransactions = transactions.filter((transaction) =>
    isInAccountActivityRange(transaction, accountActivityRange),
  );
  const accounts = collectAccounts(transactions, visibleAccountTransactions);
  const accountsCount = accounts.length;
  const activeMonthLabel = activeMonth.format("MMMM YYYY");

  useEffect(() => {
    if (settingsQuery.data) {
      settingsForm.setFieldsValue(normalizeSettings(settingsQuery.data));
    }
  }, [settingsForm, settingsQuery.data]);

  function openCreateTransaction() {
    setEditingTransaction(null);
    transactionForm.setFieldsValue({
      ...emptyTransaction,
      date: todayJournalDate(),
      postings: emptyTransaction.postings.map((posting) => ({
        ...posting,
        commodity: defaultCommodity,
      })),
    });
    setTransactionModalOpen(true);
  }

  function openEditTransaction(transaction: JournalTransaction) {
    setEditingTransaction(transaction);
    transactionForm.setFieldsValue(toTransactionInput(transaction));
    setTransactionModalOpen(true);
  }

  function submitTransaction(values: TransactionInput) {
    const normalizedValues: TransactionInput = {
      date: dayjs.isDayjs(values.date) ? values.date.format(journalDateFormat) : values.date,
      status: values.status ?? "",
      code: values.code ?? "",
      description: values.description,
      postings: (values.postings ?? [])
        .filter((posting) => posting.account.trim().length > 0)
        .map((posting) => ({
          account: posting.account,
          amount: posting.amount ?? "",
          commodity: posting.commodity ?? defaultCommodity,
          comment: posting.comment ?? "",
        })),
    };

    if (editingTransaction) {
      updateTransactionMutation.mutate({ id: editingTransaction.id, input: normalizedValues });
      return;
    }

    createTransactionMutation.mutate(normalizedValues);
  }

  function updateSettingsOnChange(_: Partial<AppSettings>, values: AppSettings) {
    updateSettingsMutation.mutate(normalizeSettings(values));
  }

  const isSavingTransaction =
    createTransactionMutation.isPending || updateTransactionMutation.isPending;

  return (
    <ConfigProvider
      theme={{
        algorithm: isDarkTheme ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: {
          borderRadius: 10,
          colorPrimary: "#1677ff",
        },
      }}
    >
      <Layout className={`app-shell ${isDarkTheme ? "theme-dark" : "theme-light"}`}>
        {contextHolder}
        <Layout.Sider className="app-sidebar" width={288}>
          <div className="sidebar-brand">
            <Typography.Title level={3}>⌘ {projectName}</Typography.Title>
          </div>

          <NavigationGroup
            items={primaryNavigation}
            activeKey={activeView}
            onSelect={(key) => setActiveView(key)}
          />
        </Layout.Sider>

        <Layout className="app-main">
          <Layout.Header className="app-header">
            <Space size="middle">
              <Typography.Title level={3}>{activeTitle}</Typography.Title>
            </Space>
            <Space>
              <Button icon={<ReloadOutlined />} onClick={() => queryClient.invalidateQueries()}>
                {t("common.refresh")}
              </Button>
              <Button type="primary" icon={<PlusOutlined />} disabled={shouldShowCourtesy} onClick={openCreateTransaction}>
                {t("transactions.newTransaction")}
              </Button>
            </Space>
          </Layout.Header>

          <Layout.Content className="app-content">
            {activeView === "settings" ? (
              <Form<AppSettings>
                form={settingsForm}
                layout="vertical"
                initialValues={activeSettings}
                onValuesChange={updateSettingsOnChange}
              >
                <Space direction="vertical" size={24} className="content-stack settings-stack">
                  <ApplicationSettingsCard />

                  <Card
                    className="settings-card"
                    title={<SectionTitle icon={<CodeOutlined />} label={t("settings.hledger")} />}
                  >
                    <Form.Item
                      label={t("settings.hledgerExecutable")}
                      name="hledgerPath"
                    >
                      <PathInput
                        placeholder={hledgerQuery.data?.resolvedPath || t("settings.hledgerExecutablePlaceholder")}
                        pickerTitle={t("settings.pickHledgerExecutable")}
                        statusAddon={(
                          <Tooltip
                            title={hledgerQuery.data?.source === "configured"
                              ? t("settings.hledgerUsingConfigured")
                              : hledgerQuery.data?.resolvedPath
                                ? t("settings.hledgerUsingDetected", { path: hledgerQuery.data.resolvedPath })
                                : t("settings.hledgerExecutableHelp")}
                          >
                            <span>
                              {hledgerQuery.data?.source === "configured"
                                ? t("settings.configured")
                                : hledgerQuery.data?.resolvedPath
                                  ? t("settings.detected")
                                  : t("settings.fallback")}
                            </span>
                          </Tooltip>
                        )}
                      />
                    </Form.Item>
                    <Descriptions column={2} size="small" className="settings-meta">
                      <Descriptions.Item label={t("settings.status")}>
                        {hledgerQuery.data?.available ? (
                          <Tag color="success">{t("common.available")}</Tag>
                        ) : (
                          <Tag color="error">{t("common.unavailable")}</Tag>
                        )}
                      </Descriptions.Item>
                      <Descriptions.Item label={t("hledger.version")}>
                        {hledgerQuery.data?.version || t("common.notDetected")}
                      </Descriptions.Item>
                    </Descriptions>
                  </Card>

                </Space>
              </Form>
            ) : shouldShowCourtesy ? (
              <CourtesyState
                reasons={courtesyReasons}
                details={journalLoadError || hledgerQuery.data?.message}
                onConfigure={() => setActiveView("settings")}
              />
            ) : activeView === "accounts" ? (
              <Space direction="vertical" size={24} className="content-stack">
                <Card
                  className="settings-card accounts-card"
                  title={t("accounts.allAccounts")}
                  extra={(
                    <Space className="accounts-range-control">
                      <Typography.Text>{t("accounts.activityRange")}</Typography.Text>
                      <Select<AccountActivityRange>
                        value={accountActivityRange}
                        onChange={setAccountActivityRange}
                        options={accountActivityRangeOptions.map((range) => ({
                          value: range,
                          label: range === "current-month"
                            ? t("accounts.currentMonth")
                            : t("accounts.lastDays", { count: Number(range) }),
                        }))}
                      />
                    </Space>
                  )}
                >
                  <Table<AccountSummary>
                    rowKey="account"
                    loading={transactionsQuery.isFetching}
                    dataSource={accounts}
                    pagination={{ pageSize: 12 }}
                    expandable={{
                      expandedRowRender: (account) => (
                        <div className="account-transactions-panel">
                          <Typography.Text className="account-transactions-title">
                            {t("accounts.accountActivity", {
                              account: account.account,
                              count: account.transactions,
                            })}
                          </Typography.Text>
                          <TransactionsTable
                            transactions={account.accountTransactions}
                            loading={transactionsQuery.isFetching}
                            powerUser={activeSettings.powerUser}
                            onEdit={openEditTransaction}
                            onDelete={(id) => deleteTransactionMutation.mutate(id)}
                          />
                        </div>
                      ),
                      rowExpandable: (account) => account.transactions > 0,
                    }}
                    columns={[
                      { title: t("transactions.account"), dataIndex: "account" },
                      {
                        title: t("accounts.transactionsCount"),
                        dataIndex: "transactions",
                        width: 180,
                        align: "right",
                        render: (count: number) => formatCount(count),
                      },
                    ]}
                  />
                </Card>
              </Space>
            ) : (
              <Space direction="vertical" size={24} className="content-stack">
                <div className="metric-grid">
                  <Card className="metric-card">
                    <span>{t("dashboard.monthlyTransactions")}</span>
                    <strong>{formatCount(monthlyTransactions.length)}</strong>
                    <p>{t("dashboard.monthlyTransactionsDescription", { month: activeMonthLabel })}</p>
                  </Card>
                  <Card className="metric-card">
                    <span>{t("dashboard.scheduledTransactions")}</span>
                    <strong>{formatCount(scheduledTransactions.length)}</strong>
                    <p>{t("dashboard.scheduledTransactionsDescription", { month: activeMonthLabel })}</p>
                  </Card>
                  <Card className="metric-card">
                    <span>{t("dashboard.activeAccounts")}</span>
                    <strong>{formatCount(accountsCount)}</strong>
                    <p>{t("dashboard.activeAccountsDescription")}</p>
                  </Card>
                </div>

                <Card className="settings-card month-toolbar-card">
                  <Space className="month-toolbar" wrap>
                    <Button icon={<LeftOutlined />} onClick={() => setActiveMonth((month) => month.subtract(1, "month"))}>
                      {t("common.previous")}
                    </Button>
                    <Typography.Title level={4}>{activeMonthLabel}</Typography.Title>
                    <Button icon={<RightOutlined />} onClick={() => setActiveMonth((month) => month.add(1, "month"))}>
                      {t("common.next")}
                    </Button>
                    <Button onClick={() => setActiveMonth(dayjs().startOf("month"))}>
                      {t("common.currentMonth")}
                    </Button>
                  </Space>
                </Card>

                <Tabs
                  className="document-tabs"
                  items={[
                    {
                      key: "executed",
                      label: t("dashboard.monthlyTransactionsTab"),
                      children: (
                        <TransactionsTable
                          transactions={monthlyTransactions}
                          loading={transactionsQuery.isFetching}
                          powerUser={activeSettings.powerUser}
                          onEdit={openEditTransaction}
                          onDelete={(id) => deleteTransactionMutation.mutate(id)}
                        />
                      ),
                    },
                    {
                      key: "scheduled",
                      label: t("dashboard.scheduledTransactionsTab"),
                      children: (
                        <TransactionsTable
                          transactions={scheduledTransactions}
                          loading={transactionsQuery.isFetching}
                          powerUser={activeSettings.powerUser}
                          onEdit={openEditTransaction}
                          onDelete={(id) => deleteTransactionMutation.mutate(id)}
                        />
                      ),
                    },
                  ]}
                />
              </Space>
            )}
          </Layout.Content>
        </Layout>

        <Modal
          title={editingTransaction ? t("transactions.editTransaction") : t("transactions.newTransaction")}
          open={isTransactionModalOpen}
          okText={editingTransaction ? t("common.save") : t("transactions.createTransaction")}
          confirmLoading={isSavingTransaction}
          onCancel={() => setTransactionModalOpen(false)}
          onOk={() => transactionForm.submit()}
        >
          <Form<TransactionInput>
            form={transactionForm}
            layout="vertical"
            initialValues={emptyTransaction}
            onFinish={submitTransaction}
          >
            <Space className="form-row" size="middle">
              <Form.Item
                label={t("transactions.date")}
                name="date"
                getValueProps={(value?: string) => ({
                  value: value ? dayjs(value, journalDateFormat) : null,
                })}
                normalize={(value: dayjs.Dayjs | null) =>
                  value ? value.format(journalDateFormat) : ""
                }
                rules={[
                  { required: true, message: t("transactions.enterTransactionDate") },
                  {
                    validator: (_, value: string) =>
                      !value || isValidJournalDate(value)
                        ? Promise.resolve()
                        : Promise.reject(new Error(t("transactions.invalidDate"))),
                  },
                ]}
              >
                <DatePicker format={journalDateFormat} className="full-width-control" />
              </Form.Item>
              <Form.Item label={t("transactions.status")} name="status">
                <Select
                  allowClear
                  className="full-width-control"
                  placeholder={t("transactions.statusPlaceholder")}
                  options={[
                    { value: "*", label: t("transactions.statusCleared") },
                    { value: "!", label: t("transactions.statusPending") },
                  ]}
                />
              </Form.Item>
              <Form.Item label={t("transactions.code")} name="code">
                <AutoComplete options={codeOptions} placeholder="(INV-001)" filterOption />
              </Form.Item>
            </Space>
            <Form.Item label={t("transactions.description")} name="description">
              <AutoComplete options={descriptionOptions} filterOption />
            </Form.Item>
            <Form.List name="postings">
              {(fields, { add, remove }) => (
                <Space direction="vertical" className="content-stack">
                  {fields.map((field) => (
                    <div key={field.key} className="posting-row">
                      <Form.Item label={t("transactions.account")} name={[field.name, "account"]}>
                        <AutoComplete options={accountOptions} placeholder="assets:bank" filterOption />
                      </Form.Item>
                      <Form.Item label={t("transactions.commodity")} name={[field.name, "commodity"]}>
                        <AutoComplete options={commodityOptions} placeholder="EUR" filterOption />
                      </Form.Item>
                      <Form.Item label={t("transactions.amount")} name={[field.name, "amount"]}>
                        <Input placeholder="25.00" />
                      </Form.Item>
                      <Button
                        danger
                        className="posting-delete-button"
                        aria-label={t("transactions.removePosting")}
                        icon={<DeleteOutlined />}
                        onClick={() => remove(field.name)}
                      />
                      <Form.Item className="posting-comment-field" name={[field.name, "comment"]}>
                        <Input placeholder={t("transactions.commentPlaceholder")} />
                      </Form.Item>
                    </div>
                  ))}
                  <Button icon={<PlusOutlined />} onClick={() => add({ account: "", amount: "", commodity: defaultCommodity, comment: "" })}>
                    {t("transactions.addPosting")}
                  </Button>
                </Space>
              )}
            </Form.List>
          </Form>
        </Modal>
      </Layout>
    </ConfigProvider>
  );
}

export default App;
