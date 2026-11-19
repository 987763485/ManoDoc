import { useEffect, useMemo, useRef, useState } from "react";
import { useI18n } from "../i18n";
import { FileIcon } from "../components/FileIcon";
import { Modal } from "../components/Modal";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  cancelBatch,
  chooseDirectory,
  chooseFiles,
  errorToAppError,
  getEngineInfo,
  onBatchProgress,
  openPath,
  retryBatchFile,
  scanBatchInputs,
  startBatch,
  type BatchConfig,
  type BatchInput,
  type BatchResult,
  type Settings,
} from "../lib/ipc";

type RowStatus =
  | "waiting"
  | "converting"
  | "cleaning"
  | "chunking"
  | "completed"
  | "failed"
  | "skipped"
  | "cancelled";

interface Row extends BatchInput {
  status: RowStatus;
  engine?: string;
  engineChoice?: string;
  fallbackReason?: string | null;
  ocrPages?: number[];
  ocrEngine?: string | null;
  tableGrade?: string | null;
  durationMs?: number;
  warnings?: string[];
  errorCode?: string | null;
  errorMessage?: string | null;
  nextStep?: string | null;
  chunks?: number;
  chars?: number;
  headings?: number;
  tableRows?: number;
  outputPath?: string | null;
}

interface RerunRequest {
  inputPaths: string[];
  config: BatchConfig;
}

const DEFAULT_CONFIG: BatchConfig = {
  preset: "standard",
  engine: "auto",
  concurrency: 4,
  cleanHeaders: true,
  cleanCjk: true,
  gradeTables: true,
  chunk: true,
  metadata: true,
  corpus: true,
  chunkSize: 1200,
  overlap: 50,
  noTextPolicy: "skip",
};

const STATUS_KEYS = {
  waiting: "status.waiting",
  converting: "status.converting",
  cleaning: "status.cleaning",
  chunking: "status.chunking",
  completed: "status.done",
  failed: "status.failed",
  skipped: "status.skipped",
  cancelled: "status.cancelled",
} as const;

export function BatchPage({
  active,
  settings,
  showToast,
  onHistoryChanged,
  rerunRequest,
  incomingPaths,
  onRerunConsumed,
  onIncomingConsumed,
}: {
  active: boolean;
  settings: Settings;
  showToast: (message: string) => void;
  onHistoryChanged: () => void;
  rerunRequest?: RerunRequest | null;
  incomingPaths?: string[] | null;
  onRerunConsumed: () => void;
  onIncomingConsumed: () => void;
}) {
  const { t } = useI18n();
  const [rows, setRows] = useState<Row[]>([]);
  const [config, setConfig] = useState<BatchConfig>(DEFAULT_CONFIG);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [filter, setFilter] = useState<"all" | "running" | "failed">("all");
  const [query, setQuery] = useState("");
  const [running, setRunning] = useState(false);
  const [percent, setPercent] = useState(0);
  const [result, setResult] = useState<BatchResult | null>(null);
  const [detail, setDetail] = useState<Row | null>(null);
  const [scanning, setScanning] = useState(false);
  const [ocrReady, setOcrReady] = useState(true);
  const batchRef = useRef<string | null>(null);

  useEffect(() => {
    void getEngineInfo()
      .then((info) => {
        setOcrReady(info.ocrReady);
        if (!info.ocrReady) {
          setConfig((current) => (current.engine === "xberg" ? { ...current, engine: "auto" } : current));
        }
      })
      .catch(() => setOcrReady(false));
  }, []);

  useEffect(() => {
    const unsubscribe = onBatchProgress((event) => {
      if (event.batchId !== batchRef.current) return;
      if (event.percent >= 0) setPercent(event.percent);
      if (!event.fileId) return;
      setRows((current) =>
        current.map((row) =>
          row.id === event.fileId
            ? { ...row, status: normalizeStatus(event.status) }
            : row,
        ),
      );
    });
    return () => {
      void unsubscribe.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!rerunRequest) return;
    onRerunConsumed();
    setConfig(rerunRequest.config);
    void scanPaths(rerunRequest.inputPaths);
    // 只在新的重跑请求到达时触发。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rerunRequest]);

  useEffect(() => {
    if (!incomingPaths?.length) return;
    onIncomingConsumed();
    void scanPaths(incomingPaths);
    // 只在新的拖入请求到达时触发。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [incomingPaths]);

  function normalizeStatus(status: string): RowStatus {
    const known = Object.keys(STATUS_KEYS) as RowStatus[];
    return known.includes(status as RowStatus) ? (status as RowStatus) : "converting";
  }

  async function scanPaths(paths: string[]) {
    if (paths.length === 0) return;
    setScanning(true);
    try {
      const scan = await scanBatchInputs(paths, settings.outputDir);
      setRows((current) => {
        const existing = new Set(current.map((row) => row.path));
        const added = scan.files
          .filter((file) => !existing.has(file.path))
          .map<Row>((file) => ({ ...file, status: "waiting" }));
        return [...current, ...added];
      });
      if (scan.skipped.length > 0) {
        showToast(`${t("batch.scanSkipped")}: ${scan.skipped[0]}`);
      }
    } catch (error) {
      showToast(errorToAppError(error).message);
    } finally {
      setScanning(false);
    }
  }

  async function addFiles() {
    const paths = await chooseFiles();
    await scanPaths(paths);
  }

  async function addFolder() {
    const path = await chooseDirectory();
    if (path) await scanPaths([path]);
  }

  function applyResult(next: BatchResult) {
    setResult(next);
    setRows((current) => {
      const map = new Map(next.files.map((file) => [file.id, file]));
      return current.map((row) => {
        const file = map.get(row.id);
        if (!file) return row;
        return {
          ...row,
          ...file,
          status: normalizeStatus(file.status),
          warnings: file.warnings,
          tableGrade: file.tableGrade,
          chunks: file.chunks,
          outputPath: file.outputPath,
        };
      });
    });
    setPercent(100);
  }

  async function start() {
    if (rows.length === 0 || running) return;
    const batchId = crypto.randomUUID();
    batchRef.current = batchId;
    setRunning(true);
    setPercent(0);
    setResult(null);
    setRows((current) =>
      current.map((row) => ({ ...row, status: row.status === "failed" ? "waiting" : row.status, warnings: [] })),
    );
    try {
      const next = await startBatch({
        batchId,
        name: deriveName(rows),
        files: rows.map((row) => ({
          id: row.id,
          path: row.path,
          name: row.name,
          relative: row.relative,
          format: row.format,
          sizeBytes: row.sizeBytes,
        })),
        config,
        outputDir: settings.outputDir,
      });
      applyResult(next);
      onHistoryChanged();
      if (next.status === "cancelled") showToast(t("batch.cancelled"));
      else if (next.status === "partial") showToast(t("batch.partial"));
      else showToast(t("batch.completed"));
    } catch (error) {
      showToast(errorToAppError(error).message);
    } finally {
      setRunning(false);
    }
  }

  async function cancel() {
    if (!batchRef.current) return;
    await cancelBatch(batchRef.current);
    showToast(t("batch.cancelled"));
  }

  async function retry(row: Row) {
    if (!result) return;
    try {
      const next = await retryBatchFile(result.batchId, row.id);
      applyResult(next);
      onHistoryChanged();
      showToast(t("batch.retryDone"));
    } catch (error) {
      showToast(errorToAppError(error).message);
    }
  }

  function remove(row: Row) {
    setRows((current) => current.filter((item) => item.id !== row.id));
  }

  function deriveName(items: Row[]) {
    const firstFolder = items[0]?.relative.split("/")[0];
    return firstFolder && items[0]?.relative.includes("/") ? firstFolder : "corpus";
  }

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return rows.filter((row) => {
      if (needle && !row.name.toLowerCase().includes(needle)) return false;
      if (filter === "failed") return row.status === "failed";
      if (filter === "running") {
        return ["converting", "cleaning", "chunking"].includes(row.status);
      }
      return true;
    });
  }, [rows, filter, query]);

  const stats = useMemo(
    () => ({
      files: rows.length,
      chunks: result?.counts.chunks ?? rows.reduce((sum, row) => sum + (row.chunks ?? 0), 0),
      warnings:
        result?.counts.warnings ?? rows.reduce((sum, row) => sum + (row.warnings?.length ?? 0), 0),
    }),
    [rows, result],
  );

  function updateConfig<K extends keyof BatchConfig>(key: K, value: BatchConfig[K]) {
    setConfig((current) => ({ ...current, [key]: value }));
  }

  function rowBadge(row: Row) {
    const tone =
      row.status === "completed"
        ? "green"
        : row.status === "failed"
          ? "red"
          : ["converting", "cleaning", "chunking"].includes(row.status)
            ? "blue"
            : "gray";
    return (
      <span className={`badge ${tone}`}>
        <span className="status-dot" />
        <span>{t(STATUS_KEYS[row.status] ?? "status.waiting")}</span>
      </span>
    );
  }

  return (
    <section className={`page${active ? " active" : ""}`} id="page-batch">
      <div className="page-head">
        <h1>{t("batch.title")}</h1>
        <p>{t("batch.sub")}</p>
        <div className="actions">
          <button className="btn secondary small" onClick={() => void addFiles()} disabled={running || scanning}>
            {t("batch.add")}
          </button>
          <button className="btn secondary small" onClick={() => void addFolder()} disabled={running || scanning}>
            {t("batch.addFolder")}
          </button>
          <button
            className="btn secondary small"
            disabled={!result?.packageDir}
            onClick={() => result?.packageDir && void openPath(`${result.packageDir}/report.csv`, true)}
          >
            {t("batch.report")}
          </button>
          <button className="btn primary small" onClick={() => void start()} disabled={running || rows.length === 0}>
            {t("batch.start")}
          </button>
        </div>
      </div>

      <div className="batch-grid">
        <div>
          <div className="stats">
            <div className="stat"><div className="n">{stats.files}</div><div className="l">{t("batch.files")}</div></div>
            <div className="stat"><div className="n">{stats.chunks}</div><div className="l">{result ? t("batch.chunksDone") : t("batch.chunks")}</div></div>
            <div className="stat"><div className="n">{stats.warnings}</div><div className="l">{t("batch.warnings")}</div></div>
          </div>

          {running ? (
            <div className="convert-progress show" role="status" aria-live="polite">
              <div className="progress-line"><span style={{ width: `${percent}%` }} /></div>
              <div className="convert-progress-row">
                <span>{t("batch.starting")}</span>
                <span className="spacer" />
                <b>{percent}%</b>
                <button className="link-btn" onClick={() => void cancel()}>{t("action.cancelBatch")}</button>
              </div>
            </div>
          ) : null}

          <div className="panel queue-panel">
            <div className="queue-head">
              <h2>{t("batch.queue")}</h2>
              <button className={`filter${filter === "all" ? " on" : ""}`} onClick={() => setFilter("all")}>{t("batch.all")}</button>
              <button className={`filter${filter === "running" ? " on" : ""}`} onClick={() => setFilter("running")}>{t("batch.running")}</button>
              <button className={`filter${filter === "failed" ? " on" : ""}`} onClick={() => setFilter("failed")}>{t("batch.failed")}</button>
              <span className="spacer" />
              <input
                className="search"
                placeholder={t("batch.search")}
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </div>
            {filtered.length === 0 ? (
              <div className="history-empty">
                <div>{rows.length === 0 ? t("batch.empty") : t("batch.emptyQueue")}</div>
                {rows.length === 0 ? <div className="hint">{t("batch.emptyHint")}</div> : null}
              </div>
            ) : (
              <table className="queue">
                <thead>
                  <tr>
                    <th>{t("th.file")}</th>
                    <th>{t("th.status")}</th>
                    <th>{t("th.engine")}</th>
                    <th>{t("th.grade")}</th>
                    <th>{t("th.time")}</th>
                    <th>{t("th.action")}</th>
                  </tr>
                </thead>
                <tbody>
                  {filtered.map((row) => (
                    <tr key={row.id}>
                      <td>
                        <span className="file-cell">
                          <FileIcon format={row.format} name={row.name} size={18} />
                          <span title={row.path}>{row.name}</span>
                        </span>
                      </td>
                      <td>{rowBadge(row)}</td>
                      <td>
                        {row.engine
                          ? `${row.engine}${row.ocrPages && row.ocrPages.length > 0 ? " · OCR" : ""}`
                          : row.status === "converting" || row.status === "cleaning" || row.status === "chunking"
                            ? "…"
                            : "—"}
                      </td>
                      <td>
                        {row.tableGrade ? (
                          <span className={`badge ${row.tableGrade === "A" ? "green" : row.tableGrade === "B" ? "amber" : "red"}`}>
                            {row.tableGrade}
                          </span>
                        ) : "—"}
                      </td>
                      <td>{row.durationMs ? `${(row.durationMs / 1000).toFixed(1)}s` : "—"}</td>
                      <td>
                        <div className="row-actions">
                          {row.status === "failed" ? (
                            <>
                              <button className="link-btn" onClick={() => void retry(row)}>{t("batch.retry")}</button>
                              <button className="link-btn" onClick={() => setDetail(row)}>{t("batch.detail")}</button>
                            </>
                          ) : row.status === "completed" ? (
                            <button className="link-btn" onClick={() => row.outputPath && void openPath(row.outputPath, true)}>{t("action.open")}</button>
                          ) : ["converting", "cleaning", "chunking"].includes(row.status) ? (
                            <button className="link-btn" onClick={() => void cancel()}>{t("action.cancel")}</button>
                          ) : (
                            <button className="link-btn" onClick={() => remove(row)} disabled={running}>{t("batch.remove")}</button>
                          )}
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>

        <aside className="panel config-panel">
          <h3>{t("batch.config")}</h3>
          <p className="hint">{t("batch.configDesc")}</p>

          <div className="field">
            <label>{t("batch.preset")}</label>
            <Select
              value={config.preset}
              onValueChange={(value) => {
                const preset =
                  value === "table"
                    ? { ...DEFAULT_CONFIG, preset: value, chunkSize: 800 }
                    : value === "corpus"
                      ? { ...DEFAULT_CONFIG, preset: value, chunkSize: 1200, overlap: 100 }
                      : { ...DEFAULT_CONFIG, preset: "standard" };
                setConfig({ ...preset, engine: config.engine, concurrency: config.concurrency });
              }}
            >
              <SelectTrigger aria-label={t("batch.preset")}><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="standard">{t("settings.presetStandard")}</SelectItem>
                <SelectItem value="table">{t("settings.presetTable")}</SelectItem>
                <SelectItem value="corpus">{t("settings.presetCorpus")}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="field">
            <label>{t("batch.outputFormat")}</label>
            <Select value="markdown" disabled>
              <SelectTrigger aria-label={t("batch.outputFormat")} title={t("batch.outputLocked")}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="markdown">Markdown</SelectItem>
                <SelectItem value="word" disabled>Word</SelectItem>
                <SelectItem value="pdf" disabled>PDF</SelectItem>
              </SelectContent>
            </Select>
            <p className="hint">{t("batch.outputLocked")}</p>
          </div>

          <div className="field">
            <label>{t("batch.engine")}</label>
            <Select value={config.engine} onValueChange={(value) => updateConfig("engine", value)}>
              <SelectTrigger aria-label={t("batch.engine")}><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="auto">{t("convert.engineAuto")}</SelectItem>
                <SelectItem value="anydoc">anydoc</SelectItem>
                <SelectItem value="xberg" disabled={!ocrReady}>
                  xberg{t("convert.engineOcrSuffix")}
                </SelectItem>
                <SelectItem value="pandoc" disabled>pandoc · {t("convert.engineLater")}</SelectItem>
              </SelectContent>
            </Select>
            {!ocrReady ? <p className="hint">{t("convert.ocrModelMissing")}</p> : null}
          </div>

          <div className="advanced">
            <button
              className="advanced-toggle"
              aria-expanded={advancedOpen}
              aria-controls="batch-advanced"
              onClick={() => setAdvancedOpen((open) => !open)}
            >
              <span>{t("batch.advanced")}</span>
              <span className="meta">
                {t("batch.advancedMeta")
                  .replace("{c}", String(config.concurrency))
                  .replace("{size}", String(config.chunkSize))
                  .replace("{overlap}", String(config.overlap))}
              </span>
              <svg viewBox="0 0 24 24"><path d="M6 9l6 6 6-6" /></svg>
            </button>
            {advancedOpen ? (
              <div className="advanced-body" id="batch-advanced">
              <div className="field">
                <label>{t("batch.concurrency")}</label>
                <Select
                  value={String(config.concurrency)}
                  onValueChange={(value) => updateConfig("concurrency", Number(value))}
                >
                  <SelectTrigger aria-label={t("batch.concurrency")}><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="1">1</SelectItem>
                    <SelectItem value="2">2</SelectItem>
                    <SelectItem value="4">4</SelectItem>
                    <SelectItem value="8">8</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="label">{t("batch.cleaning")}</div>
              <div className="rows">
                <Toggle
                  label={t("opt.headers")}
                  on={config.cleanHeaders}
                  onChange={(value) => updateConfig("cleanHeaders", value)}
                />
                <Toggle
                  label={t("opt.cjk")}
                  on={config.cleanCjk}
                  onChange={(value) => updateConfig("cleanCjk", value)}
                />
                <Toggle
                  label={t("opt.tables")}
                  on={config.gradeTables}
                  onChange={(value) => updateConfig("gradeTables", value)}
                />
                <Toggle label={t("batch.imageAssets")} on={false} disabled title={t("batch.imageLater")} onChange={() => {}} />
                <Toggle label={t("batch.chunk")} on={config.chunk} onChange={(value) => updateConfig("chunk", value)} />
                <Toggle label={t("batch.meta")} on={config.metadata} onChange={(value) => updateConfig("metadata", value)} />
                <Toggle label={t("batch.corpus")} on={config.corpus} onChange={(value) => updateConfig("corpus", value)} />
              </div>

              <div className="field">
                <label>{t("batch.chunkSize")}</label>
                <Select
                  value={String(config.chunkSize)}
                  onValueChange={(value) => updateConfig("chunkSize", Number(value))}
                >
                  <SelectTrigger aria-label={t("batch.chunkSize")}><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="800">800</SelectItem>
                    <SelectItem value="1200">1200</SelectItem>
                    <SelectItem value="2000">2000</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="field">
                <label>{t("batch.overlap")}</label>
                <Select
                  value={String(config.overlap)}
                  onValueChange={(value) => updateConfig("overlap", Number(value))}
                >
                  <SelectTrigger aria-label={t("batch.overlap")}><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="0">0</SelectItem>
                    <SelectItem value="50">50</SelectItem>
                    <SelectItem value="100">100</SelectItem>
                    <SelectItem value="200">200</SelectItem>
                  </SelectContent>
                </Select>
              </div>

                <div className="field">
                  <label>{t("batch.noTextPolicy")}</label>
                  <Select
                    value={config.noTextPolicy ?? "skip"}
                    onValueChange={(value) => updateConfig("noTextPolicy", value as BatchConfig["noTextPolicy"])}
                  >
                    <SelectTrigger aria-label={t("batch.noTextPolicy")}><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="skip">{t("batch.policySkip")}</SelectItem>
                      <SelectItem value="fail">{t("batch.policyFail")}</SelectItem>
                      <SelectItem value="ask">{t("batch.policyAsk")}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            ) : null}
          </div>

          <div className="actions">
            <button className="btn primary" onClick={() => void start()} disabled={running || rows.length === 0}>
              {t("batch.start")}
            </button>
            <button
              className="btn secondary"
              disabled={!result?.packageDir}
              onClick={() => result?.packageDir && void openPath(result.packageDir)}
            >
              {t("batch.openCorpus")}
            </button>
          </div>
        </aside>
      </div>

      <Modal open={Boolean(detail)} onClose={() => setDetail(null)} labelledBy="batch-detail-title">
        {detail ? (
          <div className="history-modal">
            <div className="history-modal-head">
              <div className="history-modal-title">
                <div className="modal-file-line">
                  <FileIcon format={detail.format} name={detail.name} size={20} />
                  <h2 id="batch-detail-title">{detail.name}</h2>
                </div>
                <div className="sub">{detail.path}</div>
              </div>
              <button className="icon-btn history-modal-close" onClick={() => setDetail(null)} title={t("action.close")} aria-label={t("action.close")}>
                <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
              </button>
            </div>
            <div className="kv">
              <b>{t("th.status")}</b><span>{t(STATUS_KEYS[detail.status] ?? "status.waiting")}</span>
              <b>{t("th.engine")}</b><span>anydoc · {detail.durationMs ?? 0}ms</span>
              <b>{t("history.stats")}</b><span>{detail.chars ?? 0} chars · {detail.headings ?? 0} headings · {detail.tableRows ?? 0} tables · {detail.chunks ?? 0} chunks</span>
              <b>{t("history.warnings")}</b><span>{detail.errorCode ?? "—"}</span>
            </div>
            {detail.errorMessage ? (
              <div className="error-card show" style={{ marginTop: 12 }}>
                <h3>{detail.errorCode}</h3>
                <p>{detail.errorMessage}</p>
                <p className="hint">{detail.nextStep}</p>
              </div>
            ) : null}
            {detail.warnings && detail.warnings.length > 0 ? (
              <ul className="batch-warnings">
                {detail.warnings.map((warning) => <li key={warning}>{warning}</li>)}
              </ul>
            ) : null}
          </div>
        ) : null}
      </Modal>
    </section>
  );
}

function Toggle({
  label,
  on,
  onChange,
  disabled,
  title,
}: {
  label: string;
  on: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
  title?: string;
}) {
  return (
    <div className="row">
      <span className="grow">{label}</span>
      <button
        type="button"
        className={`switch${on ? "" : " off"}`}
        role="switch"
        aria-checked={on}
        aria-label={label}
        title={title}
        disabled={disabled}
        onClick={() => onChange(!on)}
      />
    </div>
  );
}
