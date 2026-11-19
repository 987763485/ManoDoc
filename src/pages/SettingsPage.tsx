import { useEffect, useState } from "react";
import { useI18n } from "../i18n";
import { chooseDirectory, getEngineInfo, type EngineInfo, type Settings } from "../lib/ipc";
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

  useEffect(() => {
    void getEngineInfo().then(setEngines).catch(() => setEngines(null));
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
                <div className="info"><b>{t("settings.data")}</b><span>~/.dull_doc/settings/presets/history/logs/tmp/runtimes</span></div>
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
              <div className="engine-card">
                <div><div className="name">anydoc</div><div className="meta">{t("settings.anydocMeta")}</div></div>
                <div className="spacer"><span className="badge green">{t("settings.ready")}</span></div>
              </div>
              <div className="engine-card">
                <div><div className="name">xberg</div><div className="meta">1.4.0 · {t("settings.xbergMeta")}</div></div>
                <div className="spacer"><span className="badge green">{t("settings.ready")}</span></div>
              </div>
              <OcrModelCard showToast={showToast} />
              <div className="engine-card">
                <div><div className="name">PaddleOCR</div><div className="meta">{t("settings.paddleMeta")}</div></div>
                <div className="spacer"><span className="badge gray">{t("settings.paddleBuiltin")}</span></div>
              </div>
              <div className="engine-card">
                <div><div className="name">{t("settings.python")}</div><div className="meta">{t("settings.pythonMeta")}</div></div>
                <div className="spacer"><span className="badge gray">{t("settings.later")}</span></div>
              </div>
              <div className="engine-card">
                <div>
                  <div className="name">pandoc</div>
                  <div className="meta">{engines?.pandocVersion || t("settings.pandocMeta")}</div>
                </div>
                <div className="spacer">
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
            </section>
          ) : null}

          {tab === "output" ? (
            <section className="settings-section on" id="set-output">
              <h2>{t("settings.output")}</h2>
              <p className="intro">{t("settings.outputIntro")}</p>
              <div className="setting-row">
                <div className="info"><b>{t("settings.imageMode")}</b><span>{t("settings.imageModeDesc")}</span></div>
                <div className="control">
                  <Select value="assets" disabled>
                    <SelectTrigger aria-label={t("settings.imageMode")}><SelectValue /></SelectTrigger>
                    <SelectContent><SelectItem value="assets">{t("settings.assets")}</SelectItem></SelectContent>
                  </Select>
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.overwrite")}</b><span>{t("settings.overwriteDesc")}</span></div>
                <div className="control">
                  <Select value="autoname" disabled>
                    <SelectTrigger aria-label={t("settings.overwrite")}><SelectValue /></SelectTrigger>
                    <SelectContent><SelectItem value="autoname">{t("settings.autoname")}</SelectItem></SelectContent>
                  </Select>
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.corpus")}</b><span>{t("settings.corpusDesc")}</span></div>
                <div className="control"><span className="badge gray">{t("settings.later")}</span></div>
              </div>
            </section>
          ) : null}

          {tab === "presets" ? (
            <section className="settings-section on" id="set-presets">
              <h2>{t("settings.presets")}</h2>
              <p className="intro">{t("settings.presetsIntro")}</p>
              {[t("settings.presetStandard"), t("settings.presetTable"), t("settings.presetCorpus")].map((name) => (
                <div className="engine-card" key={name}>
                  <div><div className="name">{name}</div><div className="meta">{t("settings.later")}</div></div>
                  <div className="spacer"><span className="badge gray">{t("settings.later")}</span></div>
                </div>
              ))}
            </section>
          ) : null}

          {tab === "logs" ? (
            <section className="settings-section on" id="set-logs">
              <h2>{t("settings.logs")}</h2>
              <p className="intro">{t("settings.logsIntro")}</p>
              <div className="setting-row">
                <div className="info"><b>{t("settings.logLevel")}</b><span>info / warn / error / debug</span></div>
                <div className="control">
                  <Select defaultValue="info">
                    <SelectTrigger aria-label={t("settings.logLevel")}><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="info">info</SelectItem>
                      <SelectItem value="warn">warn</SelectItem>
                      <SelectItem value="debug">debug</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="setting-row">
                <div className="info"><b>{t("settings.logDir")}</b><span>~/.dull_doc/logs</span></div>
                <div className="control"><span className="badge gray">{t("settings.later")}</span></div>
              </div>
            </section>
          ) : null}

          {tab === "about" ? (
            <section className="settings-section on" id="set-about">
              <h2>{t("settings.about")}</h2>
              <p className="intro">{t("settings.aboutText")}</p>
              <div className="kv">
                <b>anydoc</b><span>MIT · Firecrawl</span>
                <b>xberg</b><span>MIT · Rust document intelligence</span>
                <b>PaddleOCR</b><span>Apache-2.0 · P1</span>
                <b>pandoc</b><span>GPL-2.0 · sidecar</span>
              </div>
            </section>
          ) : null}
        </div>
      </div>
    </section>
  );
}
