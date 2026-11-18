import { useEffect, useMemo, useRef, useState } from "react";
import { useI18n } from "../i18n";
import {
  cancelConversion,
  chooseFile,
  convertFile,
  errorToAppError,
  getEngineInfo,
  inspectFile,
  onFileDrop,
  onProgress,
  openPath,
  type AppError,
  type ConversionResult,
  type FileInfo,
  type Settings,
} from "../lib/ipc";
import { FileIcon } from "../components/FileIcon";
import { Modal } from "../components/Modal";

type Status = "empty" | "idle" | "converting" | "success" | "error";

const MATRIX = [
  ["Word", "Markdown", "anydoc · xberg · pandoc"],
  ["PDF", "Markdown", "anydoc · xberg · Marker"],
  ["PDF", "Word", "PDF→MD→Word · pandoc · 目录/图片保真有限"],
  ["Markdown", "Word", "pandoc"],
  ["Markdown", "PDF", "pandoc + Typst"],
  ["Markdown", "HTML / EPUB", "pandoc"],
  ["PNG / JPEG", "Markdown", "xberg · PaddleOCR · Marker"],
  ["Excel / CSV / ODS", "Markdown", "anydoc · xberg"],
  ["PowerPoint / ODP", "Markdown", "anydoc · xberg"],
  ["HTML / RTF / ODT / EPUB", "Markdown", "anydoc · xberg"],
];

export function ConvertPage({
  active,
  settings,
  showToast,
  onHistoryChanged,
  requestedPath,
  onRequestedConsumed,
  onOpenSettings,
}: {
  active: boolean;
  settings: Settings;
  showToast: (message: string) => void;
  onHistoryChanged: () => void;
  requestedPath?: string | null;
  onRequestedConsumed: () => void;
  onOpenSettings: () => void;
}) {
  const { t } = useI18n();
  const [file, setFile] = useState<FileInfo | null>(null);
  const [status, setStatus] = useState<Status>("empty");
  const [progress, setProgress] = useState({ stage: "reading", percent: 0 });
  const [result, setResult] = useState<ConversionResult | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [matrixOpen, setMatrixOpen] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [engine, setEngine] = useState<"auto" | "anydoc" | "xberg">("auto");
  const [ocrReady, setOcrReady] = useState(true);
  const taskRef = useRef<string | null>(null);
  const dropRef = useRef<(path: string) => void>(() => {});

  useEffect(() => {
    const unsubscribe = onProgress((event) => {
      if (event.taskId === taskRef.current) {
        setProgress({ stage: event.stage, percent: event.percent });
      }
    });
    return () => {
      void unsubscribe.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    void getEngineInfo()
      .then((info) => {
        setOcrReady(info.ocrReady);
        // 模型被卸载后不应继续保留不可用的显式 xberg 选择。
        if (!info.ocrReady) {
          setEngine((current) => (current === "xberg" ? "auto" : current));
        }
      })
      .catch(() => setOcrReady(false));
  }, []);

  // Tauri 默认接管原生拖放，浏览器预览仍使用 HTML5 drop。
  dropRef.current = (path) => {
    if (active) void loadPath(path);
  };
  useEffect(() => {
    const unsubscribe = onFileDrop((paths) => {
      if (paths[0]) dropRef.current(paths[0]);
    });
    return () => {
      void unsubscribe.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!requestedPath) return;
    onRequestedConsumed();
    void loadPath(requestedPath);
    // loadPath 是稳定操作；requestedPath 消费后由 App 清空。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [requestedPath]);

  async function loadPath(path: string) {
    try {
      const info = await inspectFile(path);
      setFile(info);
      setResult(null);
      setError(null);
      setStatus("idle");
      showToast(t("toast.loaded"));
    } catch (unknownError) {
      // 换文件失败时清掉旧文件，避免错误卡与旧文件信息互相矛盾。
      setFile(null);
      setResult(null);
      setError(errorToAppError(unknownError));
      setStatus("error");
    }
  }

  async function choose() {
    const path = await chooseFile();
    if (path) await loadPath(path);
  }

  async function drop(event: React.DragEvent) {
    event.preventDefault();
    setDragging(false);
    const dropped = event.dataTransfer.files[0] as (File & { path?: string }) | undefined;
    if (!dropped) return;
    if (!dropped.path) {
      showToast(t("toast.realOnly"));
      return;
    }
    await loadPath(dropped.path);
  }

  async function start() {
    if (!file) return;
    const taskId = crypto.randomUUID();
    taskRef.current = taskId;
    setStatus("converting");
    setError(null);
    setResult(null);
    setProgress({ stage: "reading", percent: 4 });
    try {
      const converted = await convertFile(file.path, taskId, engine);
      setResult(converted);
      setStatus("success");
      showToast(t("toast.converted"));
      onHistoryChanged();
    } catch (unknownError) {
      const appError = errorToAppError(unknownError);
      // 取消是用户主动动作，不按失败呈现；仍保留当前文件供再次开始。
      if (appError.code === "E_CANCELLED") {
        setStatus(file ? "idle" : "empty");
        return;
      }
      if (appError.code === "E_NEEDS_OCR") {
        // 模型可能在本次会话期间被移动或删除，错误详情比启动时的状态更权威。
        const detail = appError.detail as { ocrAvailable?: boolean } | undefined;
        if (detail?.ocrAvailable === false) setOcrReady(false);
      }
      setError(appError);
      setStatus("error");
    } finally {
      taskRef.current = null;
    }
  }

  async function cancel() {
    if (taskRef.current) await cancelConversion(taskRef.current);
    setStatus(file ? "idle" : "empty");
    showToast(t("toast.cancelled"));
  }

  const stageText = useMemo(() => {
    if (progress.stage === "reading") return t("convert.reading");
    if (progress.stage === "ocr") return t("convert.ocr");
    if (progress.stage === "writing") return t("convert.writing");
    if (progress.stage === "done") return t("convert.done");
    return t("convert.converting");
  }, [progress.stage, t]);

  const stats = result?.stats ?? { chars: 0, headings: 0, tableRows: 0 };
  const errorTitle =
    error?.code === "E_NEEDS_OCR"
      ? t("error.ocr")
      : error?.code === "E_UNSUPPORTED_FORMAT"
        ? t("error.unsupported")
        : error?.code === "E_OUTPUT_PERMISSION"
          ? t("error.permission")
          : t("error.title");

  // 常见错误由前端本地化；未知错误保留 Rust 的诊断正文，避免丢失细节。
  const ocrPages = (() => {
    if (error?.code !== "E_NEEDS_OCR") return 1;
    const detail = error.detail as { pages?: unknown[] } | undefined;
    return Array.isArray(detail?.pages) ? detail.pages.length : 1;
  })();
  const errorMessage =
    error?.code === "E_NEEDS_OCR"
      ? file?.isImage
        ? t("error.ocrImage")
        : t("error.ocrDesc", { n: ocrPages })
      : error?.code === "E_UNSUPPORTED_FORMAT"
        ? t("error.unsupportedDesc")
        : error?.code === "E_OUTPUT_PERMISSION"
          ? t("error.permissionDesc")
          : (error?.message ?? "");
  const errorNext =
    error?.code === "E_NEEDS_OCR"
      ? ocrReady
        ? t("error.nextOcrReady")
        : t("error.nextOcr")
      : error?.code === "E_UNSUPPORTED_FORMAT"
        ? t("error.nextUnsupported")
        : error?.code === "E_OUTPUT_PERMISSION"
          ? t("error.nextPermission")
          : (error?.nextStep ?? "");

  return (
    <section className={`page${active ? " active" : ""}`} id="page-convert">
      <div className="page-head">
        <h1>{t("convert.title")}</h1>
        <p>{t("convert.sub")}</p>
        <div className="actions">
          <button className="btn secondary small" onClick={() => setMatrixOpen(true)}>
            {t("palette.pages") === "Go to" ? "Supported directions" : "支持的方向"}
          </button>
          <button className="btn secondary small" onClick={() => openPath(settings.outputDir)}>
            {t("action.openFolder")}
          </button>
        </div>
      </div>

      <div className="convert-grid">
        <aside className="panel file-panel">
          <div className="field-label">{t("convert.file")}</div>

          {status === "empty" ? (
            <div
              className={`file-empty show${dragging ? " dragover" : ""}`}
              id="file-empty"
              onDragOver={(event) => {
                event.preventDefault();
                setDragging(true);
              }}
              onDragLeave={() => setDragging(false)}
              onDrop={drop}
              onClick={choose}
              role="button"
              tabIndex={0}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") void choose();
              }}
            >
              <span>{t("convert.empty")}</span>
              <span className="hint">{t("convert.emptyHint")}</span>
              <button className="link-btn" onClick={(event) => { event.stopPropagation(); void choose(); }}>
                {t("convert.choose")}
              </button>
            </div>
          ) : null}

          {file && status !== "empty" ? (
            <div className="file-card">
              <FileIcon format={file.format} name={file.name} />
              <div className="file-info">
                <div className="file-name" title={file.name}>{file.name}</div>
                <div className="file-meta">
                  {Math.max(1, Math.round(file.sizeBytes / 1024))} KB · {file.format}
                </div>
              </div>
              <button className="file-remove" aria-label={t("action.cancel")} onClick={() => { setFile(null); setStatus("empty"); setResult(null); setError(null); }}>
                <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
              </button>
            </div>
          ) : null}

          {file ? (
            <>
              <div className="route-row">
                <div className="field">
                  <label>{t("convert.direction")}</label>
                  <div className="route-control">
                    <span className="route-source">
                      <FileIcon format={file.format} size={18} />
                      <span>{file.format}</span>
                    </span>
                    <span aria-hidden="true">→</span>
                    <span>Markdown</span>
                  </div>
                </div>
                <div className="field">
                  <label>{t("convert.engine")}</label>
                  <select
                    value={engine}
                    onChange={(event) => setEngine(event.target.value as "auto" | "anydoc" | "xberg")}
                  >
                    <option value="auto">{t("convert.engineAuto")}</option>
                    <option value="anydoc">anydoc</option>
                    <option value="xberg" disabled={!ocrReady}>
                      xberg{t("convert.engineOcrSuffix")}
                    </option>
                    <option disabled>pandoc · {t("convert.engineLater")}</option>
                  </select>
                  {!ocrReady ? <span className="hint">{t("convert.ocrModelMissing")}</span> : null}
                </div>
              </div>
              <div className="field">
                <label>{t("convert.preset")}</label>
                <select defaultValue="standard">
                  <option value="standard">{t("convert.presetStandard")}</option>
                  <option value="table">{t("settings.presetTable")}</option>
                  <option value="knowledge">{t("settings.presetCorpus")}</option>
                </select>
              </div>
              <p className="hint">{t("convert.imageLimit")}</p>

              <div className="actions">
                <button className="btn primary" id="start-convert" disabled={status === "converting"} onClick={() => void start()}>
                  {t("convert.start")}
                </button>
                <button className="btn secondary" onClick={() => openPath(settings.outputDir)}>
                  {t("action.openFolder")}
                </button>
              </div>
            </>
          ) : null}

          {status === "converting" ? (
            <div className="convert-progress show" id="convert-progress" role="status" aria-live="polite">
              <div className="progress-line"><span style={{ width: `${progress.percent}%` }} /></div>
              <div className="convert-progress-row">
                <span>{stageText}</span>
                <span className="spacer" />
                <b>{progress.percent}%</b>
                <button className="link-btn" onClick={() => void cancel()}>{t("action.cancel")}</button>
              </div>
            </div>
          ) : null}

          {status === "success" && result ? (
            <div className="success-card show" id="convert-success">
              <h3>{t("convert.done")}</h3>
              <p>
                <b>{result.outputName}</b> · {t("convert.resultHint", { chars: stats.chars, headings: stats.headings, tables: stats.tableRows })}
              </p>
              <p className="hint" id="convert-engine-info">
                {t("convert.engineUsed")}: <b>{result.engineUsed}</b>
                {result.ocrPages.length > 0
                  ? ` · ${t("convert.ocrPages", { n: result.ocrPages.length })}`
                  : ""}
                {result.fallbackReason ? ` · ${t("convert.fallback", { reason: result.fallbackReason })}` : ""}
              </p>
              {result.warnings.length > 0 ? <p className="hint">{result.warnings.join(" · ")}</p> : null}
              <div className="actions">
                <button className="btn primary small" onClick={() => openPath(result.outputPath, true)}>{t("action.openOutput")}</button>
                <button className="btn secondary small" onClick={() => openPath(settings.outputDir)}>{t("action.openFolder")}</button>
              </div>
            </div>
          ) : null}

          {status === "error" && error ? (
            <div className="error-card show" id="convert-error">
              <h3>{errorTitle}</h3>
              <p><b>{error.code}</b> · {errorMessage}</p>
              <p className="hint">{t("error.next")}: {errorNext}</p>
              <div className="actions">
                <button className="btn secondary small" onClick={() => void choose()}>{t("action.chooseFile")}</button>
                {error.code === "E_OUTPUT_PERMISSION" ? (
                  <button className="btn ghost small" onClick={onOpenSettings}>{t("action.openSettings")}</button>
                ) : null}
                {error.code === "E_NEEDS_OCR" && !ocrReady ? (
                  <button className="btn secondary small" id="install-ocr-link" onClick={onOpenSettings}>
                    {t("convert.installOcr")}
                  </button>
                ) : null}
                <button className="btn ghost small" onClick={() => setStatus(file ? "idle" : "empty")}>{t("action.close")}</button>
              </div>
            </div>
          ) : null}
        </aside>
      </div>

      <Modal open={matrixOpen} onClose={() => setMatrixOpen(false)} labelledBy="matrix-title">
        <div className="history-modal matrix-modal">
          <div className="history-modal-head">
            <div className="history-modal-title">
              <h2 id="matrix-title">{t("palette.pages") === "Go to" ? "Supported directions" : "支持的方向"}</h2>
              <div className="sub">{t("convert.unsupported")}</div>
            </div>
            <button className="icon-btn history-modal-close" onClick={() => setMatrixOpen(false)} title={t("action.close")} aria-label={t("action.close")}>
              <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
            </button>
          </div>
          <div className="matrix-list">
            {MATRIX.map(([from, to, engines]) => (
              <div className="matrix-row" key={`${from}-${to}`}>
                <div className="matrix-route">
                  <span className="matrix-formats"><b>{from}</b></span>
                  <span>→</span>
                  <span className="matrix-formats"><b>{to}</b></span>
                </div>
                <div className="matrix-meta">{engines}</div>
              </div>
            ))}
          </div>
          <p className="matrix-note">
            {t("palette.pages") === "Go to"
              ? "PDF → Word uses PDF → Markdown → Word; TOC/image fidelity is limited. Markdown → PDF uses pandoc + Typst."
              : "PDF → Word 走 PDF → Markdown → Word，目录与图片保真有限；Markdown → PDF 由 pandoc + Typst 生成。"}
          </p>
        </div>
      </Modal>
    </section>
  );
}
