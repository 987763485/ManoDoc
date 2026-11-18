import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useI18n } from "../i18n";
import type { HistoryTask } from "../lib/ipc";
import { FileIcon } from "./FileIcon";

export type PageId = "convert" | "batch" | "history" | "settings";

interface Command {
  key: string;
  group: "pages" | "actions" | "recent";
  label: string;
  hint?: string;
  icon?: "page" | "theme" | "language" | "folder";
  fileFormat?: string;
  run: () => void;
}

const ICONS: Record<string, ReactNode> = {
  page: (
    <svg viewBox="0 0 24 24"><path d="M7 3h7l4 4v14H7z" /><path d="M14 3v4h4" /></svg>
  ),
  theme: (
    <svg viewBox="0 0 24 24"><path d="M20 14.5A8 8 0 0 1 9.5 4 7 7 0 1 0 20 14.5z" /></svg>
  ),
  language: (
    <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" /><path d="M4 12h16M12 4c2.5 2.4 2.5 13.6 0 16M12 4c-2.5 2.4-2.5 13.6 0 16" /></svg>
  ),
  folder: (
    <svg viewBox="0 0 24 24"><path d="M3 7h7l2 2h9v10H3z" /></svg>
  ),
};

/** Cmd+K 命令面板：页面、动作与最近文件共用同一套键盘导航。 */
export function CommandPalette({
  open,
  onClose,
  onNavigate,
  onToggleTheme,
  onToggleLanguage,
  onOpenOutput,
  recent,
  onPickRecent,
}: {
  open: boolean;
  onClose: () => void;
  onNavigate: (page: PageId) => void;
  onToggleTheme: () => void;
  onToggleLanguage: () => void;
  onOpenOutput: () => void;
  recent: HistoryTask[];
  onPickRecent: (path: string) => void;
}) {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActive(0);
    // 面板打开后让输入框立刻可输入，Esc 仍由输入框处理。
    window.setTimeout(() => inputRef.current?.focus(), 0);
  }, [open]);

  const commands = useMemo<Command[]>(() => {
    const pages: Command[] = [
      { key: "page:convert", group: "pages", label: t("nav.convert"), hint: "1", icon: "page", run: () => onNavigate("convert") },
      { key: "page:batch", group: "pages", label: t("nav.batch"), hint: "2", icon: "page", run: () => onNavigate("batch") },
      { key: "page:history", group: "pages", label: t("nav.history"), hint: "3", icon: "page", run: () => onNavigate("history") },
      { key: "page:settings", group: "pages", label: t("nav.settings"), hint: "4", icon: "page", run: () => onNavigate("settings") },
    ];
    const actions: Command[] = [
      { key: "action:theme", group: "actions", label: t("palette.theme"), icon: "theme", run: onToggleTheme },
      { key: "action:language", group: "actions", label: t("palette.language"), icon: "language", run: onToggleLanguage },
      { key: "action:output", group: "actions", label: t("palette.output"), icon: "folder", run: onOpenOutput },
    ];
    const files: Command[] = recent.slice(0, 6).map((task) => ({
      key: `file:${task.id}`,
      group: "recent",
      label: task.inputName,
      hint: task.direction,
      fileFormat: task.inputName.split(".").pop(),
      run: () => onPickRecent(task.inputPath),
    }));
    return [...pages, ...actions, ...files];
  }, [t, recent, onNavigate, onToggleTheme, onToggleLanguage, onOpenOutput, onPickRecent]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return commands;
    return commands.filter((command) =>
      `${command.label} ${command.hint ?? ""}`.toLowerCase().includes(needle),
    );
  }, [commands, query]);

  useEffect(() => {
    if (active >= filtered.length) setActive(Math.max(0, filtered.length - 1));
  }, [filtered.length, active]);

  if (!open) return null;

  function run(command: Command) {
    onClose();
    command.run();
  }

  const groups: Array<{ id: Command["group"]; label: string }> = [
    { id: "pages", label: t("palette.pages") },
    { id: "actions", label: t("palette.actions") },
    { id: "recent", label: t("palette.recent") },
  ];

  return (
    <div
      className="palette open"
      role="dialog"
      aria-modal="true"
      aria-label={t("palette.title")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="palette-box" onMouseDown={(event) => event.stopPropagation()}>
        <input
          ref={inputRef}
          className="palette-input"
          placeholder={t("palette.placeholder")}
          value={query}
          autoComplete="off"
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              onClose();
            } else if (event.key === "ArrowDown") {
              event.preventDefault();
              setActive((index) => Math.min(index + 1, filtered.length - 1));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActive((index) => Math.max(index - 1, 0));
            } else if (event.key === "Enter" && filtered[active]) {
              event.preventDefault();
              run(filtered[active]);
            }
          }}
        />
        <div className="palette-list">
          {filtered.length === 0 ? <div className="palette-empty">{t("palette.empty")}</div> : null}
          {groups.map((group) => {
            const items = filtered.filter((command) => command.group === group.id);
            if (items.length === 0) return null;
            return (
              <div key={group.id}>
                <div className="palette-group">{group.label}</div>
                {items.map((command) => {
                  const index = filtered.indexOf(command);
                  return (
                    <button
                      key={command.key}
                      className={`palette-item${index === active ? " on" : ""}`}
                      onMouseEnter={() => setActive(index)}
                      onClick={() => run(command)}
                    >
                      <span className="ic">
                        {command.fileFormat ? (
                          <FileIcon format={command.fileFormat} name={command.label} size={17} />
                        ) : (
                          ICONS[command.icon ?? "page"]
                        )}
                      </span>
                      <span>{command.label}</span>
                      {command.hint ? <span className="hint">{command.hint}</span> : null}
                    </button>
                  );
                })}
              </div>
            );
          })}
        </div>
        <div className="palette-foot">
          <span><b>↑↓</b> {t("palette.select")}</span>
          <span><b>Enter</b> {t("palette.run")}</span>
          <span><b>Esc</b> {t("palette.close")}</span>
          <span style={{ marginLeft: "auto" }}>DullDoc</span>
        </div>
      </div>
    </div>
  );
}
