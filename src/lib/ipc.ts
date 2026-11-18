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
}

export interface FileInfo {
  name: string;
  path: string;
  sizeBytes: number;
  format: string;
  direction: string;
}

export interface ConversionResult {
  taskId: string;
  outputPath: string;
  outputName: string;
  stats: TaskStats;
  durationMs: number;
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

export async function getEngineInfo(): Promise<{ anydocVersion: string; ready: boolean }> {
  if (!isTauri()) return { anydocVersion: "0.2.4", ready: true };
  return invoke("get_engine_info");
}

export async function inspectFile(path: string): Promise<FileInfo> {
  if (!isTauri()) {
    const name = path.split("/").pop() ?? "示例文件.docx";
    return {
      name,
      path,
      sizeBytes: 958 * 1024,
      format: name.split(".").pop()?.toUpperCase() ?? "DOCX",
      direction: "DOCX → Markdown",
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
        extensions: ["doc", "docx", "odt", "rtf", "epub", "pdf", "ppt", "pptx", "xls", "xlsx", "csv"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseDirectory(): Promise<string | null> {
  if (!isTauri()) return "~/Documents/DullDoc Output";
  const selected = await open({ multiple: false, directory: true });
  return typeof selected === "string" ? selected : null;
}

export async function convertFile(inputPath: string, taskId: string): Promise<ConversionResult> {
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
    };
    writeMockHistory([task, ...readMockHistory()]);
    return { taskId, outputPath, outputName: "GB-T-28448.md", stats, durationMs: 920 };
  }
  return invoke<ConversionResult>("convert_file", { inputPath, taskId });
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
