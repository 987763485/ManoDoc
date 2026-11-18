import { useEffect, useMemo, useState } from "react";
import { useI18n } from "../i18n";
import {
  errorToAppError,
  getOcrStatus,
  installOcrModels,
  onOcrModelProgress,
  type AppError,
  type OcrProgress,
  type OcrStatus,
} from "../lib/ipc";

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 MB";
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/**
 * OCR 模型状态与安装卡片。
 * 安装必须是用户显式动作；组件只负责展示来源、体积、进度与失败原因。
 */
export function OcrModelCard({ showToast }: { showToast: (message: string) => void }) {
  const { t } = useI18n();
  const [status, setStatus] = useState<OcrStatus | null>(null);
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<OcrProgress | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  useEffect(() => {
    void getOcrStatus()
      .then(setStatus)
      .catch((unknown) => setError(errorToAppError(unknown)));
  }, []);

  useEffect(() => {
    const unsubscribe = onOcrModelProgress((event) => setProgress(event));
    return () => {
      void unsubscribe.then((fn) => fn());
    };
  }, []);

  async function install() {
    setInstalling(true);
    setError(null);
    setProgress(null);
    try {
      const next = await installOcrModels();
      setStatus(next);
      showToast(t("settings.ocrInstalled"));
    } catch (unknown) {
      setError(errorToAppError(unknown));
    } finally {
      setInstalling(false);
      setProgress(null);
    }
  }

  const badge = useMemo(() => {
    if (!status) return { label: t("settings.ocrChecking"), className: "gray" };
    if (status.ready) return { label: t("settings.ocrInstalled"), className: "green" };
    if (status.partial) return { label: t("settings.ocrPartial"), className: "amber" };
    return { label: t("settings.ocrMissing"), className: "gray" };
  }, [status, t]);

  const percent = progress?.percent ?? 0;

  return (
    <div className="engine-card ocr-model-card" id="ocr-model-card">
      <div>
        <div className="name">PaddleOCR · PP-OCRv6 small</div>
        <div className="meta">
          {status
            ? `${formatBytes(status.installedBytes)} / ${formatBytes(status.requiredBytes)} · ${t("settings.ocrSource")}`
            : t("settings.ocrChecking")}
        </div>
        {status ? <div className="meta ocr-model-dir">{status.modelDir}</div> : null}
        {installing ? (
          <div className="ocr-model-progress" role="status" aria-live="polite">
            <div className="progress-line">
              <span style={{ width: `${percent}%` }} />
            </div>
            <div className="meta">
              {progress
                ? `${progress.file} · ${percent}%`
                : t("settings.ocrInstalling")}
            </div>
          </div>
        ) : null}
        {error ? (
          <div className="meta ocr-model-error" role="alert">
            {error.code} · {error.message}
            <br />
            {error.nextStep}
          </div>
        ) : null}
      </div>
      <div className="spacer ocr-model-actions">
        <span className={`badge ${badge.className}`}>{badge.label}</span>
        {status && !status.ready ? (
          <button className="btn secondary small" onClick={() => void install()} disabled={installing}>
            {installing ? t("settings.ocrInstalling") : t("settings.ocrInstall")}
          </button>
        ) : null}
      </div>
    </div>
  );
}
