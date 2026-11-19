import { useEffect, useState } from "react";
import { useI18n } from "../i18n";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  chooseDirectory,
  deletePreset,
  duplicatePreset,
  exportDiagnostics,
  exportPresets,
  getEngineInfo,
  getLogsDir,
  importPresets,
  listPresets,
  openPath,
  savePreset,
  type EngineInfo,
  type Preset,
  type Settings,
} from "../lib/ipc";
import { OcrModelCard } from "../components/OcrModelCard";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

type Tab = "general" | "engines" | "output" | "presets" | "logs" | "about";

const tabs: Tab[] = ["general", "engines", "output", "presets", "logs", "about"];

const tabKeys = {
  general: "settings.general",
  engines: "settings.engines",
  output: "settings.output",
  presets: "settings.presets",
  logs: "settings.logs",
  about: "settings.about",
} as const;

export function SettingsPage({
  active,
  settings,
  onUpdate,
  showToast,
}: {
  active: boolean;
  settings: Settings;
  onUpdate: (patch: Partial<Settings>) => Promise<void>;
  showToast: (message: string) => void;
}) {
  const { t } = useI18n();
  const [tab, setTab] = useState<Tab>("general");
  const [engines, setEngines] = useState<EngineInfo | null>(null);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [editingPreset, setEditingPreset] = useState<string | null>(null);
  const [editName, setEditName] = useState("");

  useEffect(() => {
    void getEngineInfo().then(setEngines).catch(() => setEngines(null));
    void listPresets().then(setPresets).catch(() => setPresets([]));
  }, []);

  async function changeOutputDir() {
    const path = await chooseDirectory();
    if (path) {
      await onUpdate({ outputDir: path });
      showToast(t("toast.settingsSaved"));
    }
  }

  return (
    <section className={`page${active ? " active" : ""}`} id="page-settings">
      <div className="page-head">
        <h1>{t("settings.title")}</h1>
        <p>{t("settings.sub")}</p>
      </div>
      <div className="settings-grid">
        <aside className="panel settings-nav" role="tablist" aria-label={t("settings.title")}>
          {tabs.map((item) => (
            <button
              key={item}
              className={tab === item ? "on" : ""}
              role="tab"
              aria-selected={tab === item}
              onClick={() => setTab(item)}
            >
              {t(tabKeys[item])}
            </button>
          ))}
        </aside>
        <div className="panel settings-panel">
          {tab === "general" ? (
            <section className="settings-section on" id="set-general">
              <h2>{t("settings.general")}</h2>
              <p className="intro">{t("settings.langDesc")}</p>
              <div className="setting-row">
                <div className="info"><b>{t("settings.lang")}</b><span>{t("settings.langDesc")}</span></div>
                <div className="control">
                  <div className="seg">
                    <button className={settings.language === "zh" ? "on" : ""} onClick={() => void onUpdate({ language: "zh" })}>中文</button>
                    <button className={settings.language === "en" ? "on" : ""} onClick={() => void onUpdate({ language: "en" })}>English</button>
                  </div>
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.theme")}</b><span>{t("settings.themeDesc")}</span></div>
                <div className="control">
                  <Select
                    value={settings.theme}
                    onValueChange={(value) => void onUpdate({ theme: value as Settings["theme"] })}
                  >
                    <SelectTrigger aria-label={t("settings.theme")}><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="system">{t("theme.system")}</SelectItem>
                      <SelectItem value="light">{t("theme.light")}</SelectItem>
                      <SelectItem value="dark">{t("theme.dark")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.outputDir")}</b><span>{settings.outputDir}</span></div>
                <div className="control"><button className="btn secondary small" onClick={() => void changeOutputDir()}>{t("action.change")}</button></div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.data")}</b><span>~/.manodoc/settings/presets/history/logs/tmp/runtimes</span></div>
                <div className="control"><span className="badge green">{t("settings.granted")}</span></div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.folderAccess")}</b><span>{t("settings.folderAccessDesc")}</span></div>
                <div className="control"><span className="badge green">{t("settings.granted")}</span></div>
              </div>
            </section>
          ) : null}

          {tab === "engines" ? (
            <section className="settings-section on" id="set-engines">
              <h2>{t("settings.engines")}</h2>
              <p className="intro">{t("settings.enginesIntro")}</p>

              <div className="engine-group">
                <h3>{t("settings.groupEngines")}</h3>
                <p className="engine-group-intro">{t("settings.groupEnginesDesc")}</p>
                <div className="engine-card">
                  <div><div className="name">anydoc</div><div className="meta">{t("settings.anydocMeta")}</div></div>
                  <div className="spacer"><span className="badge green">{t("settings.ready")}</span></div>
                </div>
                <div className="engine-card">
                  <div><div className="name">xberg</div><div className="meta">1.4.0 · {t("settings.xbergMeta")}</div></div>
                  <div className="spacer"><span className="badge green">{t("settings.ready")}</span></div>
                </div>
                <div className="engine-card">
                  <div>
                    <div className="name">pandoc</div>
                    <div className="meta">{engines?.pandocVersion || t("settings.pandocMeta")}</div>
                    {settings.pandocPath ? (
                      <div className="meta engine-path">{settings.pandocPath}</div>
                    ) : null}
                  </div>
                  <div className="spacer actions">
                    <button
                      className="btn ghost small"
                      onClick={async () => {
                        const picked = await open({ multiple: false, directory: false });
                        if (typeof picked !== "string") return;
                        await onUpdate({ pandocPath: picked });
                        setEngines(await getEngineInfo());
                      }}
                    >
                      {t("settings.pandocCustom")}
                    </button>
                    <span className={`badge ${engines?.pandocReady ? "green" : "gray"}`}>
                      {engines?.pandocReady ? t("settings.ready") : t("settings.later")}
                    </span>
                  </div>
                </div>
                <div className="engine-card">
                  <div>
                    <div className="name">Typst</div>
                    <div className="meta">{engines?.typstVersion || t("settings.typstMeta")}</div>
                  </div>
                  <div className="spacer">
                    <span className={`badge ${engines?.typstReady ? "green" : "gray"}`}>
                      {engines?.typstReady ? t("settings.ready") : t("settings.later")}
                    </span>
                  </div>
                </div>
              </div>

              <div className="engine-group">
                <h3>{t("settings.groupOcr")}</h3>
                <p className="engine-group-intro">{t("settings.groupOcrDesc")}</p>
                <OcrModelCard showToast={showToast} />
                <div className="engine-card">
                  <div><div className="name">PaddleOCR</div><div className="meta">{t("settings.paddleMeta")}</div></div>
                  <div className="spacer"><span className="badge gray">{t("settings.paddleBuiltin")}</span></div>
                </div>
              </div>

              <div className="engine-group">
                <h3>{t("settings.groupRuntime")}</h3>
                <p className="engine-group-intro">{t("settings.groupRuntimeDesc")}</p>
                <div className="engine-card">
                  <div><div className="name">{t("settings.python")}</div><div className="meta">{t("settings.pythonMeta")}</div></div>
                  <div className="spacer"><span className="badge gray">{t("settings.later")}</span></div>
                </div>
              </div>
            </section>
          ) : null}

          {tab === "output" ? (
            <section className="settings-section on" id="set-output">
              <h2>{t("settings.output")}</h2>
              <p className="intro">{t("settings.outputIntro")}</p>
              <div className="setting-row">
                <div className="info"><b>{t("settings.imageMode")}</b><span>{t("settings.imageModeDesc")}</span></div>
                <div className="control">
                  <Select value={settings.imageMode} onValueChange={(value) => void onUpdate({ imageMode: value as Settings["imageMode"] })}>
                    <SelectTrigger aria-label={t("settings.imageMode")}><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="assets">{t("settings.assets")}</SelectItem>
                      <SelectItem value="alt">{t("settings.imageAlt")}</SelectItem>
                      <SelectItem value="base64">{t("settings.imageBase64")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.overwrite")}</b><span>{t("settings.overwriteDesc")}</span></div>
                <div className="control">
                  <Select value={settings.overwrite} onValueChange={(value) => void onUpdate({ overwrite: value as Settings["overwrite"] })}>
                    <SelectTrigger aria-label={t("settings.overwrite")}><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="rename">{t("settings.overwriteRename")}</SelectItem>
                      <SelectItem value="overwrite">{t("settings.overwriteReplace")}</SelectItem>
                      <SelectItem value="skip">{t("settings.overwriteSkip")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.historyLimit")}</b><span>{t("settings.historyLimitDesc")}</span></div>
                <div className="control">
                  <input
                    type="number"
                    min={10}
                    max={500}
                    value={settings.historyLimit}
                    onChange={(event) => void onUpdate({ historyLimit: Math.max(10, Math.min(500, Number(event.target.value) || 50)) })}
                  />
                </div>
              </div>
            </section>
          ) : null}

          {tab === "presets" ? (
            <section className="settings-section on" id="set-presets">
              <h2>{t("settings.presets")}</h2>
              <p className="intro">{t("settings.presetsIntro")}</p>
              <div className="actions" style={{ marginBottom: 12 }}>
                <button
                  className="btn ghost small"
                  onClick={async () => {
                    const source = await open({ multiple: false });
                    if (typeof source === "string") setPresets(await importPresets(source));
                  }}
                >
                  {t("settings.presetImport")}
                </button>
                <button
                  className="btn ghost small"
                  onClick={async () => {
                    const target = await save({ defaultPath: "manodoc-presets.json" });
                    if (target) await exportPresets(target);
                  }}
                >
                  {t("settings.presetExport")}
                </button>
              </div>
              {presets.map((preset) => (
                <div className="engine-card" key={preset.id}>
                  {editingPreset === preset.id ? (
                    <>
                      <input
                        value={editName}
                        aria-label={t("settings.presetName")}
                        onChange={(event) => setEditName(event.target.value)}
                      />
                      <div className="spacer actions">
                        <button
                          className="btn secondary small"
                          disabled={!editName.trim()}
                          onClick={async () => {
                            setPresets(await savePreset({ ...preset, name: editName.trim() }));
                            setEditingPreset(null);
                          }}
                        >
                          {t("action.save")}
                        </button>
                        <button className="btn ghost small" onClick={() => setEditingPreset(null)}>
                          {t("action.cancel")}
                        </button>
                      </div>
                    </>
                  ) : (
                    <>
                      <div><div className="name">{preset.name}</div><div className="meta">{preset.builtin ? t("settings.presetBuiltin") : t("settings.presetCustom")}</div></div>
                      <div className="spacer actions">
                        {!preset.builtin ? (
                          <button
                            className="btn ghost small"
                            onClick={() => {
                              setEditingPreset(preset.id);
                              setEditName(preset.name);
                            }}
                          >
                            {t("settings.presetEdit")}
                          </button>
                        ) : null}
                        <button className="btn ghost small" onClick={async () => setPresets(await duplicatePreset(preset.id))}>{t("settings.presetDuplicate")}</button>
                        {!preset.builtin ? (
                          <button className="btn ghost small" onClick={async () => setPresets(await deletePreset(preset.id))}>{t("action.delete")}</button>
                        ) : null}
                      </div>
                    </>
                  )}
                </div>
              ))}
            </section>
          ) : null}

          {tab === "logs" ? (
            <section className="settings-section on" id="set-logs">
              <h2>{t("settings.logs")}</h2>
              <p className="intro">{t("settings.logsIntro")}</p>
              <div className="setting-row">
                <div className="info"><b>{t("settings.logLevel")}</b><span>{t("settings.logLevelDesc")}</span></div>
                <div className="control">
                  <Select value={settings.logLevel} onValueChange={(value) => void onUpdate({ logLevel: value as Settings["logLevel"] })}>
                    <SelectTrigger aria-label={t("settings.logLevel")}><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="error">error</SelectItem>
                      <SelectItem value="warn">warn</SelectItem>
                      <SelectItem value="info">info</SelectItem>
                      <SelectItem value="debug">debug</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.logRetention")}</b><span>{t("settings.logRetentionDesc")}</span></div>
                <div className="control">
                  <input
                    type="number"
                    min={1}
                    max={365}
                    value={settings.logRetentionDays}
                    onChange={(event) => void onUpdate({ logRetentionDays: Math.max(1, Math.min(365, Number(event.target.value) || 7)) })}
                  />
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.logDir")}</b><span>~/.manodoc/logs</span></div>
                <div className="control actions">
                  <button
                    className="btn secondary small"
                    onClick={async () => {
                      const dir = await getLogsDir();
                      await openPath(dir);
                    }}
                  >
                    {t("settings.openLogs")}
                  </button>
                  <button
                    className="btn secondary small"
                    onClick={async () => {
                      const target = await save({ defaultPath: "manodoc-diagnostics.zip" });
                      if (!target) return;
                      await exportDiagnostics(target);
                      showToast(t("toast.diagnosticsSaved"));
                    }}
                  >
                    {t("settings.exportDiagnostics")}
                  </button>
                </div>
              </div>
            </section>
          ) : null}

          {tab === "about" ? (
            <section className="settings-section on" id="set-about">
              <h2>{t("settings.about")}</h2>
              <p className="intro">{t("settings.aboutText")}</p>
              <div className="kv">
                <b>anydoc</b><span>MIT · Firecrawl · 文档转换内核</span>
                <b>xberg</b><span>MIT · Rust document intelligence · OCR / 表格</span>
                <b>PaddleOCR</b><span>Apache-2.0 · OCR 组件（P1）</span>
                <b>Typst</b><span>Apache-2.0 · PDF 排版</span>
                <b>pandoc</b><span>GPL-2.0 · 独立 sidecar 进程调用</span>
                <b>{t("settings.licenseTitle")}</b><span>MIT License</span>
              </div>
              <p className="hint">{t("settings.pandocLicense")}</p>
            </section>
          ) : null}
        </div>
      </div>
    </section>
  );
}
