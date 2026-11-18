import { useState } from "react";
import { useI18n } from "../i18n";
import { clearHistory, openPath, type HistoryTask } from "../lib/ipc";
import { FileIcon } from "../components/FileIcon";
import { Modal } from "../components/Modal";

export function HistoryPage({
  active,
  tasks,
  onRefresh,
  showToast,
  onRerun,
}: {
  active: boolean;
  tasks: HistoryTask[];
  onRefresh: () => Promise<void>;
  showToast: (message: string) => void;
  onRerun: (path: string) => void;
}) {
  const { t, lang } = useI18n();
  const [selected, setSelected] = useState<HistoryTask | null>(null);
  const [clearOpen, setClearOpen] = useState(false);

  function formatTime(iso: string) {
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) return "—";
    return date.toLocaleTimeString(lang === "zh" ? "zh-CN" : "en-US", {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  async function confirmClear() {
    await clearHistory();
    await onRefresh();
    setClearOpen(false);
    showToast(t("history.cleared"));
  }

  async function copyParams(task: HistoryTask) {
    const text = JSON.stringify(
      { input: task.inputPath, direction: task.direction, engine: task.engine },
      null,
      2,
    );
    try {
      await navigator.clipboard.writeText(text);
      showToast(t("toast.copied"));
    } catch {
      showToast(text);
    }
  }

  return (
    <section className={`page${active ? " active" : ""}`} id="page-history">
      <div className="page-head">
        <h1>{t("history.title")}</h1>
        <p>{t("history.sub")}</p>
        <div className="actions">
          <button className="btn ghost small" id="history-clear" onClick={() => setClearOpen(true)}>
            {t("action.clear")}
          </button>
        </div>
      </div>
      <h2 className="sr-only">{t("history.detail")}</h2>
      <div className="history-grid">
        <div className="panel history-list" id="history-list">
          {tasks.length === 0 ? (
            <div className="history-empty">{t("history.empty")}</div>
          ) : (
            tasks.map((task) => (
              <div
                className="history-row"
                key={task.id}
                role="button"
                tabIndex={0}
                aria-haspopup="dialog"
                onClick={() => setSelected(task)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    setSelected(task);
                  }
                }}
              >
                <div className="time">{formatTime(task.createdAt)}</div>
                <div>
                  <div className="name">
                    <FileIcon format={task.inputName.split(".").pop()} name={task.inputName} size={18} />
                    <span>{task.inputName}</span>
                  </div>
                  <div className="sub">{task.direction} · {task.engine}</div>
                </div>
                <div>
                  <span className={`badge ${task.status === "completed" ? "green" : "red"}`}>
                    <span className="status-dot" />
                    <span>{task.status === "completed" ? t("status.completed") : t("status.failed")}</span>
                  </span>
                </div>
                <div>{task.stats.headings} {t("unit.headings")}</div>
                <div>
                  <button
                    className="link-btn"
                    onClick={(event) => {
                      event.stopPropagation();
                      if (task.outputPath) void openPath(task.outputPath, true);
                      else showToast(task.errorCode ?? t("history.noOutput"));
                    }}
                  >
                    {task.status === "completed" ? t("action.open") : t("action.retry")}
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      <Modal open={Boolean(selected)} onClose={() => setSelected(null)} labelledBy="detail-title">
        {selected ? (
          <div className="history-modal">
            <div className="history-modal-head">
              <div className="history-modal-title">
                <div className="modal-file-line">
                  <FileIcon format={selected.inputName.split(".").pop()} name={selected.inputName} size={20} />
                  <h2 id="detail-title">{selected.inputName}</h2>
                </div>
                <div className="sub">
                  {selected.direction} · {new Date(selected.createdAt).toLocaleString(lang === "zh" ? "zh-CN" : "en-US")}
                </div>
              </div>
              <button className="icon-btn history-modal-close" onClick={() => setSelected(null)} title={t("action.close")} aria-label={t("action.close")}>
                <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
              </button>
            </div>
            <div className="kv">
              <b>{t("history.preset")}</b><span>{t("settings.presetStandard")}</span>
              <b>{t("history.direction")}</b><span>{selected.direction}</span>
              <b>{t("history.target")}</b><span>Markdown</span>
              <b>{t("history.engine")}</b><span>{selected.engine} · {selected.durationMs}ms</span>
              <b>{t("history.output")}</b><span>{selected.outputPath ?? t("history.noOutput")}</span>
              <b>{t("history.stats")}</b><span>{selected.stats.chars} {t("unit.chars")} · {selected.stats.headings} {t("unit.headings")} · {selected.stats.tableRows} {t("unit.tables")}</span>
              <b>{t("history.warnings")}</b><span>{selected.errorCode ?? "—"}</span>
            </div>
            <div className="actions">
              <button className="btn primary small" onClick={() => { onRerun(selected.inputPath); setSelected(null); }}>
                {t("history.rerun")}
              </button>
              <button className="btn secondary small" onClick={() => selected.outputPath && void openPath(selected.outputPath, true)}>
                {t("action.openOutput")}
              </button>
              <button className="btn ghost small" onClick={() => void copyParams(selected)}>
                {t("history.copy")}
              </button>
            </div>
          </div>
        ) : null}
      </Modal>

      <Modal open={clearOpen} onClose={() => setClearOpen(false)} labelledBy="clear-title">
        <div className="history-modal">
          <div className="history-modal-head">
            <div className="history-modal-title">
              <h2 id="clear-title">{t("history.clearTitle")}</h2>
              <div className="sub">{t("history.clearDesc")}</div>
            </div>
            <button className="icon-btn history-modal-close" onClick={() => setClearOpen(false)} title={t("action.close")} aria-label={t("action.close")}>
              <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
            </button>
          </div>
          <div className="actions">
            <button className="btn ghost small" onClick={() => setClearOpen(false)}>{t("action.cancel")}</button>
            <button className="btn danger small" onClick={() => void confirmClear()}>{t("history.clearConfirm")}</button>
          </div>
        </div>
      </Modal>
    </section>
  );
}
