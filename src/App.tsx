import { useCallback, useEffect, useRef, useState } from "react";
import { I18nProvider, useI18n } from "./i18n";
import {
  getSettings,
  saveSettings,
  listHistory,
  openPath,
  errorToAppError,
  type HistoryTask,
  type Settings,
  type BatchConfig,
} from "./lib/ipc";
import { ConvertPage } from "./pages/ConvertPage";
import { BatchPage } from "./pages/BatchPage";
import { HistoryPage } from "./pages/HistoryPage";
import { SettingsPage } from "./pages/SettingsPage";
import { CommandPalette, type PageId } from "./components/CommandPalette";

// 首屏使用与 Rust 默认值一致的设置，避免主题闪动；随后以磁盘配置为准。
const DEFAULT_SETTINGS: Settings = {
  version: 1,
  language: "zh",
  theme: "system",
  outputDir: "~/Documents/DullDoc Output",
};

export default function App() {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [page, setPage] = useState<PageId>("convert");
  const [history, setHistory] = useState<HistoryTask[]>([]);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [toast, setToast] = useState("");
  const [requestedPath, setRequestedPath] = useState<string | null>(null);
  const [requestedTarget, setRequestedTarget] = useState<string | null>(null);
  const [batchRerun, setBatchRerun] = useState<{
    inputPaths: string[];
    config: BatchConfig;
  } | null>(null);
  const toastTimer = useRef<number | null>(null);
  const lastFocus = useRef<HTMLElement | null>(null);

  const showToast = useCallback((message: string) => {
    setToast(message);
    if (toastTimer.current) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(""), 2200);
  }, []);

  const refreshHistory = useCallback(async () => {
    try {
      setHistory(await listHistory());
    } catch (error) {
      showToast(errorToAppError(error).message);
    }
  }, [showToast]);

  useEffect(() => {
    void (async () => {
      try {
        setSettings(await getSettings());
      } catch (error) {
        showToast(errorToAppError(error).message);
      }
    })();
    void refreshHistory();
    return () => {
      if (toastTimer.current) window.clearTimeout(toastTimer.current);
    };
  }, [refreshHistory, showToast]);

  // 跟随系统时订阅外观变化，切到显式主题后解除订阅。
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme =
        settings.theme === "system" ? (media.matches ? "dark" : "light") : settings.theme;
    };
    apply();
    if (settings.theme !== "system") return;
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [settings.theme]);

  const changePalette = useCallback((open: boolean) => {
    if (open) {
      lastFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      setPaletteOpen(true);
    } else {
      setPaletteOpen(false);
      // 等面板卸载后再把焦点还给触发按钮。
      window.requestAnimationFrame(() => lastFocus.current?.focus());
    }
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        changePalette(true);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [changePalette]);

  async function updateSettings(patch: Partial<Settings>) {
    const next = { ...settings, ...patch };
    setSettings(next);
    try {
      setSettings(await saveSettings(next));
    } catch (error) {
      showToast(errorToAppError(error).message);
    }
  }

  function toggleTheme() {
    const resolved =
      settings.theme === "system"
        ? window.matchMedia("(prefers-color-scheme: dark)").matches
          ? "dark"
          : "light"
        : settings.theme;
    void updateSettings({ theme: resolved === "dark" ? "light" : "dark" });
  }

  function toggleLanguage() {
    void updateSettings({ language: settings.language === "zh" ? "en" : "zh" });
  }

  function pickRecent(path: string, target?: string) {
    setRequestedPath(path);
    setRequestedTarget(target ?? null);
    setPage("convert");
  }

  function rerunBatch(task: HistoryTask) {
    if (!task.inputPaths?.length || !task.params) return;
    // 旧批量记录可能缺少后续新增字段，重跑时用当前默认值补齐。
    const params = task.params;
    const config: BatchConfig = {
      preset: params.preset ?? "standard",
      engine: params.engine ?? "auto",
      concurrency: params.concurrency ?? 4,
      cleanHeaders: params.cleanHeaders ?? true,
      cleanCjk: params.cleanCjk ?? true,
      gradeTables: params.gradeTables ?? true,
      chunk: params.chunk ?? true,
      metadata: params.metadata ?? true,
      corpus: params.corpus ?? true,
      chunkSize: params.chunkSize ?? 1200,
      overlap: params.overlap ?? 50,
    };
    setBatchRerun({ inputPaths: task.inputPaths, config });
    setPage("batch");
  }

  return (
    <I18nProvider lang={settings.language}>
      <AppShell
        settings={settings}
        page={page}
        history={history}
        toast={toast}
        paletteOpen={paletteOpen}
        requestedPath={requestedPath}
        requestedTarget={requestedTarget}
        batchRerun={batchRerun}
        onPage={setPage}
        onPalette={changePalette}
        onUpdateSettings={updateSettings}
        onToggleTheme={toggleTheme}
        onToggleLanguage={toggleLanguage}
        onRefreshHistory={refreshHistory}
        onShowToast={showToast}
        onPickRecent={pickRecent}
        onRerunBatch={rerunBatch}
        onBatchRerunConsumed={() => setBatchRerun(null)}
        onRequestedConsumed={() => setRequestedPath(null)}
      />
    </I18nProvider>
  );
}

function AppShell({
  settings,
  page,
  history,
  toast,
  paletteOpen,
  requestedPath,
  requestedTarget,
  batchRerun,
  onPage,
  onPalette,
  onUpdateSettings,
  onToggleTheme,
  onToggleLanguage,
  onRefreshHistory,
  onShowToast,
  onPickRecent,
  onRerunBatch,
  onBatchRerunConsumed,
  onRequestedConsumed,
}: {
  settings: Settings;
  page: PageId;
  history: HistoryTask[];
  toast: string;
  paletteOpen: boolean;
  requestedPath: string | null;
  requestedTarget: string | null;
  batchRerun: { inputPaths: string[]; config: BatchConfig } | null;
  onPage: (page: PageId) => void;
  onPalette: (open: boolean) => void;
  onUpdateSettings: (patch: Partial<Settings>) => Promise<void>;
  onToggleTheme: () => void;
  onToggleLanguage: () => void;
  onRefreshHistory: () => Promise<void>;
  onShowToast: (message: string) => void;
  onPickRecent: (path: string, target?: string) => void;
  onRerunBatch: (task: HistoryTask) => void;
  onBatchRerunConsumed: () => void;
  onRequestedConsumed: () => void;
}) {
  const { t } = useI18n();
  const [sidebarOpen, setSidebarOpen] = useState(() => {
    try {
      return window.localStorage.getItem("dulldoc.sidebar") !== "0";
    } catch {
      return true;
    }
  });

  useEffect(() => {
    try {
      window.localStorage.setItem("dulldoc.sidebar", sidebarOpen ? "1" : "0");
    } catch {
      // 隐私模式下 localStorage 不可写；折叠状态只影响本次会话。
    }
  }, [sidebarOpen]);

  async function openOutput() {
    try {
      await openPath(settings.outputDir);
    } catch {
      onShowToast(t("toast.openFailed"));
    }
  }

  return (
    <div className={sidebarOpen ? "app" : "app sidebar-collapsed"}>
      <header className="titlebar" data-tauri-drag-region>
        <div className="tb-left" data-tauri-drag-region>
          <button
            className="tb-icon"
            title={t("titlebar.sidebar")}
            aria-label={t("titlebar.sidebar")}
            aria-pressed={!sidebarOpen}
            onClick={() => setSidebarOpen((open) => !open)}
          >
            <svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M9 4v16" /></svg>
          </button>
          <button
            className="tb-icon"
            title={t("titlebar.search")}
            aria-label={t("palette.open")}
            onClick={(event) => {
              event.currentTarget.focus();
              onPalette(true);
            }}
          >
            <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7" /><path d="M16.5 16.5L21 21" /></svg>
          </button>
        </div>
        <div className="tb-title" data-tauri-drag-region>{t("app.name")}</div>
        <div className="tb-right" data-tauri-drag-region>
          <div className="seg" aria-label={t("settings.lang")}>
            <button className={settings.language === "zh" ? "on" : ""} onClick={() => settings.language !== "zh" && onToggleLanguage()}>中文</button>
            <button className={settings.language === "en" ? "on" : ""} onClick={() => settings.language !== "en" && onToggleLanguage()}>EN</button>
          </div>
          <button className="icon-btn" title={t("palette.theme")} aria-label={t("palette.theme")} onClick={onToggleTheme}>
            <svg className="sun" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="4" />
              <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
            </svg>
            <svg className="moon" viewBox="0 0 24 24"><path d="M20 14.5A8 8 0 0 1 9.5 4 7 7 0 1 0 20 14.5z" /></svg>
          </button>
        </div>
      </header>

      <aside className="sidebar">
        <nav className="nav" aria-label={t("nav.convert")}>
          <button className={page === "convert" ? "active" : ""} onClick={() => onPage("convert")}>
            <svg viewBox="0 0 24 24"><path d="M7 3h7l4 4v14H7z" /><path d="M14 3v4h4" /><path d="M10 12h5M10 16h5" /></svg>
            <span>{t("nav.convert")}</span>
          </button>
          <button className={page === "batch" ? "active" : ""} onClick={() => onPage("batch")}>
            <svg viewBox="0 0 24 24"><path d="M4 7h16M4 12h16M4 17h16" /><path d="M7 4h10v3H7z" /></svg>
            <span>{t("nav.batch")}</span>
          </button>
          <button className={page === "history" ? "active" : ""} onClick={() => onPage("history")}>
            <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" /><path d="M12 8v5l3 2" /></svg>
            <span>{t("nav.history")}</span>
          </button>
          <button className={page === "settings" ? "active" : ""} onClick={() => onPage("settings")}>
            <svg viewBox="0 0 24 24"><path d="M4 7h10M18 7h2M4 17h2M10 17h10" /><circle cx="16" cy="7" r="2" /><circle cx="8" cy="17" r="2" /></svg>
            <span>{t("nav.settings")}</span>
          </button>
        </nav>
        <div className="side-foot">
          <span className="live" />
          <span>{t("side.offline")}</span>
          <button
            className="kbd"
            onClick={(event) => {
              event.currentTarget.focus();
              onPalette(true);
            }}
            aria-label={t("palette.open")}
          >
            ⌘K
          </button>
        </div>
      </aside>

      <div className="main">
        <main className="content">
          <ConvertPage
            active={page === "convert"}
            settings={settings}
            showToast={onShowToast}
            onHistoryChanged={() => void onRefreshHistory()}
            requestedPath={requestedPath}
        requestedTarget={requestedTarget}
            onRequestedConsumed={onRequestedConsumed}
            onOpenSettings={() => onPage("settings")}
          />
          <BatchPage
            active={page === "batch"}
            settings={settings}
            showToast={onShowToast}
            onHistoryChanged={() => void onRefreshHistory()}
            rerunRequest={batchRerun}
            onRerunConsumed={onBatchRerunConsumed}
          />
          <HistoryPage
            active={page === "history"}
            tasks={history}
            onRefresh={onRefreshHistory}
            showToast={onShowToast}
            onRerun={onPickRecent}
            onRerunBatch={onRerunBatch}
          />
          <SettingsPage
            active={page === "settings"}
            settings={settings}
            onUpdate={onUpdateSettings}
            showToast={onShowToast}
          />
        </main>
      </div>

      <CommandPalette
        open={paletteOpen}
        onClose={() => onPalette(false)}
        onNavigate={onPage}
        onToggleTheme={onToggleTheme}
        onToggleLanguage={onToggleLanguage}
        onOpenOutput={openOutput}
        recent={history}
        onPickRecent={onPickRecent}
      />

      <div className={`toast${toast ? " show" : ""}`} role="status" aria-live="polite">
        {toast}
      </div>
    </div>
  );
}
