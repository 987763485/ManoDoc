import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

export type Language = "zh" | "en";
export type Theme = "system" | "light" | "dark";

export interface Settings {
  version: number;
  language: Language;
  theme: Theme;
  outputDir: string;
}

export interface TaskStats {
  chars: number;
  headings: number;
  tableRows: number;
}

export interface HistoryParams extends Partial<BatchConfig> {
  engineChoice?: string;
  engineUsed?: string;
  fallbackReason?: string | null;
  ocrPages?: number[];
  ocrEngine?: string | null;
  method?: string;
  pages?: number | null;
  target?: string;
}

export interface HistoryTask {
  id: string;
  createdAt: string;
  inputPath: string;
  inputName: string;
  direction: string;
  engine: string;
  status: "completed" | "failed" | string;
  outputPath: string | null;
  errorCode: string | null;
  stats: TaskStats;
  durationMs: number;
  kind?: "single" | "batch" | string;
  inputCount?: number;
  inputPaths?: string[];
  outputDir?: string | null;
  counts?: BatchCounts | null;
  params?: HistoryParams | null;
}

export interface BatchCounts {
  success: number;
  skipped: number;
  failed: number;
  warnings: number;
  chunks: number;
}

export interface BatchConfig {
  preset: string;
  engine: string;
  concurrency: number;
  cleanHeaders: boolean;
  cleanCjk: boolean;
  gradeTables: boolean;
  chunk: boolean;
  metadata: boolean;
  corpus: boolean;
  chunkSize: number;
  overlap: number;
}

export interface BatchInput {
  id: string;
  path: string;
  name: string;
  relative: string;
  format: string;
  sizeBytes: number;
}

export interface BatchScan {
  files: BatchInput[];
  skipped: string[];
}

export interface BatchFileResult extends BatchInput {
  hash: string;
  status: string;
  engine: string;
  tableGrade: string | null;
  warnings: string[];
  errorCode: string | null;
  errorMessage: string | null;
  nextStep: string | null;
  chars: number;
  headings: number;
  tableRows: number;
  chunks: number;
  durationMs: number;
  outputPath: string | null;
}

export interface BatchResult {
  batchId: string;
  name: string;
  status: string;
  packageDir: string;
  files: BatchFileResult[];
  counts: BatchCounts;
  durationMs: number;
}

export interface BatchRequest {
  batchId: string;
  name?: string;
  files: BatchInput[];
  config: BatchConfig;
  outputDir: string;
}

export interface BatchProgress {
  batchId: string;
  fileId: string | null;
  status: string;
  completed: number;
  total: number;
  percent: number;
}

export interface FileInfo {
  name: string;
  path: string;
  sizeBytes: number;
  format: string;
  direction: string;
  isImage: boolean;
  targets: string[];
  defaultTarget: string;
}

export interface ConversionResult {
  taskId: string;
  outputPath: string;
  outputName: string;
  stats: TaskStats;
  durationMs: number;
  target: string;
  direction: string;
  engineUsed: string;
  engineChoice: string;
  fallbackReason: string | null;
  ocrPages: number[];
  ocrEngine: string | null;
  warnings: string[];
}

export interface EngineInfo {
  anydocVersion: string;
  xbergVersion: string;
  ready: boolean;
  ocrReady: boolean;
  ocrModelDir: string;
  pandocReady: boolean;
  pandocVersion: string;
  typstReady: boolean;
  typstVersion: string;
}

export interface OcrModelFile {
  relativePath: string;
  sizeBytes: number;
  sha256: string;
  sourceUrl: string;
  present: boolean;
}

export interface OcrStatus {
  engine: string;
  engineVersion: string;
  ready: boolean;
  partial: boolean;
  installedBytes: number;
  requiredBytes: number;
  modelDir: string;
  source: string;
  files: OcrModelFile[];
}

export interface OcrProgress {
  file: string;
  downloaded: number;
  total: number;
  percent: number;
  fileIndex: number;
  fileCount: number;
}

export interface ProgressEvent {
  taskId: string;
  stage: string;
  percent: number;
}

export interface AppError {
  code: string;
  message: string;
  detail?: unknown;
  nextStep: string;
}

const isTauri = () => "__TAURI_INTERNALS__" in window;

const SETTINGS_KEY = "dulldoc.settings";
const HISTORY_KEY = "dulldoc.history";

const defaultSettings: Settings = {
  version: 1,
  language: "zh",
  theme: "system",
  outputDir: "~/Documents/DullDoc Output",
};

function readMockSettings(): Settings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return defaultSettings;
    return { ...defaultSettings, ...JSON.parse(raw) } as Settings;
  } catch {
    return defaultSettings;
  }
}

function readMockHistory(): HistoryTask[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    return raw ? (JSON.parse(raw) as HistoryTask[]) : [];
  } catch {
    return [];
  }
}

function writeMockHistory(tasks: HistoryTask[]) {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(tasks.slice(0, 50)));
}

export async function getSettings(): Promise<Settings> {
  if (!isTauri()) return readMockSettings();
  return invoke<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<Settings> {
  if (!isTauri()) {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
    return settings;
  }
  return invoke<Settings>("save_settings", { settings });
}

export async function getEngineInfo(): Promise<EngineInfo> {
  if (!isTauri()) {
    return {
      anydocVersion: "0.2.4",
      xbergVersion: "1.4.0",
      ready: true,
      ocrReady: true,
      ocrModelDir: "~/.dull_doc/models/paddleocr",
      pandocReady: true,
      pandocVersion: "pandoc 3.14",
      typstReady: true,
      typstVersion: "typst 0.15.1",
    };
  }
  return invoke<EngineInfo>("get_engine_info");
}

export async function getOcrStatus(): Promise<OcrStatus> {
  if (!isTauri()) {
    return {
      engine: "xberg",
      engineVersion: "1.4.0",
      ready: true,
      partial: false,
      installedBytes: 31_186_579,
      requiredBytes: 31_186_579,
      modelDir: "~/.dull_doc/models/paddleocr",
      source: "https://huggingface.co/xberg-io/paddleocr-onnx-models",
      files: [],
    };
  }
  return invoke<OcrStatus>("get_ocr_status");
}

export async function installOcrModels(): Promise<OcrStatus> {
  if (!isTauri()) {
    await new Promise((resolve) => setTimeout(resolve, 600));
    return getOcrStatus();
  }
  return invoke<OcrStatus>("install_ocr_models");
}

export async function onOcrModelProgress(
  handler: (event: OcrProgress) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  return listen<OcrProgress>("ocr-model-progress", (event) => handler(event.payload));
}

export async function inspectFile(path: string): Promise<FileInfo> {
  if (!isTauri()) {
    const name = path.split("/").pop() ?? "示例文件.docx";
    const ext = name.split(".").pop()?.toLowerCase() ?? "docx";
    const format = ext.toUpperCase();
    return {
      name,
      path,
      sizeBytes: 958 * 1024,
      format,
      direction: `${format} → Markdown`,
      isImage: ["png", "jpg", "jpeg"].includes(ext),
      targets: ext === "md" || ext === "markdown"
        ? ["docx", "pdf", "html", "epub"]
        : ext === "pdf"
          ? ["markdown", "docx"]
          : ["markdown"],
      defaultTarget: ext === "md" || ext === "markdown" ? "docx" : "markdown",
    };
  }
  return invoke<FileInfo>("inspect_file", { path });
}

export async function chooseFile(): Promise<string | null> {
  if (!isTauri()) return "~/Documents/示例/GB-T-28448.docx";
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "支持的文件",
        extensions: [
          "doc", "docx", "odt", "rtf", "epub", "pdf", "ppt", "pptx", "xls", "xlsx", "csv",
          "png", "jpg", "jpeg", "md", "markdown", "html", "htm", "mhtml",
        ],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseFiles(): Promise<string[]> {
  if (!isTauri()) {
    return [
      "~/Documents/示例/GB-T-28448.docx",
      "~/Documents/示例/等级保护测评2.0要求指标清单v1.xlsx",
    ];
  }
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [
      {
        name: "支持的文件",
        extensions: [
          "doc", "docx", "odt", "rtf", "epub", "pdf", "ppt", "pptx", "xls", "xlsx", "csv",
          "png", "jpg", "jpeg", "md", "markdown", "html", "htm", "mhtml",
        ],
      },
    ],
  });
  return Array.isArray(selected) ? selected : selected ? [selected] : [];
}

export async function chooseDirectory(): Promise<string | null> {
  if (!isTauri()) return "~/Documents/DullDoc Output";
  const selected = await open({ multiple: false, directory: true });
  return typeof selected === "string" ? selected : null;
}

export async function convertFile(
  inputPath: string,
  taskId: string,
  engineChoice: string = "auto",
  targetFormat: string = "markdown",
): Promise<ConversionResult> {
  if (!isTauri()) {
    await new Promise((resolve) => setTimeout(resolve, 900));
    const stats = { chars: 53210, headings: 49, tableRows: 15 };
    const outputPath = "~/Documents/DullDoc Output/GB-T-28448.md";
    const task: HistoryTask = {
      id: taskId,
      createdAt: new Date().toISOString(),
      inputPath,
      inputName: inputPath.split("/").pop() ?? "GB-T-28448.docx",
      direction: "DOCX → Markdown",
      engine: "anydoc",
      status: "completed",
      outputPath,
      errorCode: null,
      stats,
      durationMs: 920,
      kind: "single",
      inputCount: 1,
      inputPaths: [inputPath],
      outputDir: null,
      counts: null,
      params: { engineChoice, engineUsed: "anydoc" },
    };
    writeMockHistory([task, ...readMockHistory()]);
    return {
      taskId,
      outputPath,
      outputName: "GB-T-28448.md",
      stats,
      durationMs: 920,
      target: "Markdown",
      direction: "DOCX → Markdown",
      engineUsed: "anydoc",
      engineChoice,
      fallbackReason: null,
      ocrPages: [],
      ocrEngine: null,
      warnings: [],
    };
  }
  return invoke<ConversionResult>("convert_file", {
    inputPath,
    taskId,
    engineChoice,
    targetFormat,
  });
}

export async function cancelConversion(taskId: string): Promise<void> {
  if (!isTauri()) return;
  await invoke("cancel_conversion", { taskId });
}

export async function listHistory(): Promise<HistoryTask[]> {
  if (!isTauri()) return readMockHistory();
  return invoke<HistoryTask[]>("list_history");
}

export async function clearHistory(): Promise<void> {
  if (!isTauri()) {
    localStorage.setItem(HISTORY_KEY, JSON.stringify([]));
    return;
  }
  await invoke("clear_history");
}

export async function openPath(path: string, reveal = false): Promise<void> {
  if (!isTauri()) return;
  await invoke("open_path", { path, reveal });
}

export async function onProgress(
  handler: (event: ProgressEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  return listen<ProgressEvent>("conversion-progress", (event) => handler(event.payload));
}

/** 桌面端由 Tauri 接管文件拖放；浏览器预览仍走 HTML5 drop。 */
export async function onFileDrop(
  handler: (paths: string[]) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  return getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type === "drop") handler(event.payload.paths);
  });
}

const FORMAT_MAP: Record<string, string> = {
  doc: "DOC", docx: "DOCX", odt: "ODT", rtf: "RTF", epub: "EPUB", pdf: "PDF",
  ppt: "PPT", pptx: "PPTX", xls: "XLSX", xlsx: "XLSX", csv: "CSV", ods: "ODS", odp: "ODP",
};

export async function scanBatchInputs(paths: string[], outputDir: string): Promise<BatchScan> {
  if (!isTauri()) {
    const files: BatchInput[] = [];
    const skipped: string[] = [];
    for (const path of paths) {
      const name = path.split("/").pop() ?? "未命名文件";
      const ext = name.split(".").pop()?.toLowerCase() ?? "";
      const format = FORMAT_MAP[ext];
      if (!format) {
        skipped.push(`跳过不支持的文件: ${name}`);
        continue;
      }
      files.push({
        id: `mock-${files.length}`,
        path,
        name,
        relative: name,
        format,
        sizeBytes: 512 * 1024,
      });
    }
    return { files, skipped };
  }
  return invoke<BatchScan>("scan_batch_inputs", { paths, outputDir });
}

export async function startBatch(request: BatchRequest): Promise<BatchResult> {
  if (!isTauri()) {
    await new Promise((resolve) => setTimeout(resolve, 650));
    const packageDir = `~/Documents/DullDoc Output/${request.name ?? "corpus"}-20261119-010000`;
    const files: BatchFileResult[] = request.files.map((input, index) => ({
      ...input,
      hash: "mock-hash",
      status: "completed",
      engine: "anydoc",
      tableGrade: "A",
      warnings: [],
      errorCode: null,
      errorMessage: null,
      nextStep: null,
      chars: 1200 + index * 300,
      headings: 5 + index,
      tableRows: 2,
      chunks: request.config.chunk ? 4 : 0,
      durationMs: 80 + index * 20,
      outputPath: `${packageDir}/mirror/${input.relative.replace(/\.[^.]+$/, ".md")}`,
    }));
    const counts: BatchCounts = {
      success: files.length,
      skipped: 0,
      failed: 0,
      warnings: 0,
      chunks: files.reduce((sum, file) => sum + file.chunks, 0),
    };
    const result: BatchResult = {
      batchId: request.batchId,
      name: request.name ?? "corpus",
      status: "completed",
      packageDir,
      files,
      counts,
      durationMs: 650,
    };
    const task: HistoryTask = {
      id: request.batchId,
      createdAt: new Date().toISOString(),
      inputPath: files[0]?.path ?? "",
      inputName: result.name,
      direction: "批量 → Markdown",
      engine: "anydoc",
      status: "completed",
      outputPath: packageDir,
      errorCode: null,
      stats: {
        chars: counts.success * 1200,
        headings: counts.success * 5,
        tableRows: counts.success * 2,
      },
      durationMs: 650,
      kind: "batch",
      inputCount: files.length,
      inputPaths: request.files.map((file) => file.path),
      outputDir: packageDir,
      counts,
      params: request.config,
    };
    writeMockHistory([task, ...readMockHistory()]);
    return result;
  }
  return invoke<BatchResult>("start_batch", { request });
}

export async function cancelBatch(batchId: string): Promise<void> {
  if (!isTauri()) return;
  await invoke("cancel_batch", { batchId });
}

export async function retryBatchFile(batchId: string, fileId: string): Promise<BatchResult> {
  if (!isTauri()) throw new Error("浏览器预览不支持重试");
  return invoke<BatchResult>("retry_batch_file", { batchId, fileId });
}

export async function onBatchProgress(
  handler: (event: BatchProgress) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  return listen<BatchProgress>("batch-progress", (event) => handler(event.payload));
}

export function errorToAppError(error: unknown): AppError {
  if (typeof error === "object" && error && "code" in error) {
    return error as AppError;
  }
  return {
    code: "E_UNKNOWN",
    message: String(error),
    nextStep: "重试或复制诊断信息",
  };
}
