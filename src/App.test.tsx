import { beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach } from "vitest";
import App from "./App";

function setup() {
  return render(<App />);
}

beforeEach(() => {
  localStorage.clear();
  document.documentElement.dataset.theme = "light";
});

afterEach(() => {
  cleanup();
});

describe("应用壳与导航", () => {
  it("默认进入文档转换页并可切换四页", async () => {
    setup();
    expect(screen.getByRole("heading", { name: "文档转换" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "转换历史" }));
    expect(screen.getByRole("heading", { name: "转换历史" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "系统设置" }));
    expect(screen.getByRole("heading", { name: "系统设置" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "语料制作" }));
    expect(screen.getByRole("heading", { name: "语料制作" })).toBeInTheDocument();
  });

  it("切换英文后界面即时更新并写入设置", async () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: "EN" }));

    expect(screen.getByRole("heading", { name: "Convert" })).toBeInTheDocument();
    await waitFor(() => {
      const saved = JSON.parse(localStorage.getItem("dulldoc.settings") ?? "{}");
      expect(saved.language).toBe("en");
    });
  });

  it("切换主题为深色并持久化", async () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: "切换主题" }));

    await waitFor(() => {
      expect(document.documentElement.dataset.theme).toBe("dark");
      const saved = JSON.parse(localStorage.getItem("dulldoc.settings") ?? "{}");
      expect(saved.theme).toBe("dark");
    });
  });

  it("Cmd+K 搜索并进入设置，Esc 关闭后焦点返回触发按钮", async () => {
    setup();

    fireEvent.keyDown(window, { key: "k", metaKey: true });
    const input = screen.getByPlaceholderText("搜索命令、页面或最近文件…");
    fireEvent.change(input, { target: { value: "系统设置" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(await screen.findByRole("heading", { name: "系统设置" })).toBeInTheDocument();

    const trigger = screen.getByTitle("搜索与命令（⌘K）");
    fireEvent.click(trigger);
    const reopened = screen.getByPlaceholderText("搜索命令、页面或最近文件…");
    fireEvent.keyDown(reopened, { key: "Escape" });

    await waitFor(() => {
      expect(screen.queryByPlaceholderText("搜索命令、页面或最近文件…")).not.toBeInTheDocument();
      expect(document.activeElement).toBe(trigger);
    });
  });
});

describe("单文件转换与历史", () => {
  it("选择文件后完成转换并进入历史", async () => {
    setup();

    fireEvent.click(screen.getByRole("button", { name: /^选择文件$/ }));
    expect(await screen.findByText("GB-T-28448.docx")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "开始转换" }));
    expect(
      await screen.findByRole("heading", { name: "转换完成" }, { timeout: 3000 }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "转换历史" }));
    const historyPage = document.getElementById("page-history") as HTMLElement;
    expect(await within(historyPage).findByText("GB-T-28448.docx")).toBeInTheDocument();
    expect(within(historyPage).getByText("完成")).toBeInTheDocument();
  });

  it("清空历史需要弹窗确认，确认后列表为空", async () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: /^选择文件$/ }));
    await screen.findByText("GB-T-28448.docx");
    fireEvent.click(screen.getByRole("button", { name: "开始转换" }));
    await screen.findByRole("heading", { name: "转换完成" }, { timeout: 3000 });

    fireEvent.click(screen.getByRole("button", { name: "转换历史" }));
    const historyPage = document.getElementById("page-history") as HTMLElement;
    await within(historyPage).findByText("GB-T-28448.docx");
    fireEvent.click(within(historyPage).getByRole("button", { name: "清空历史" }));

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByRole("heading", { name: "清空转换历史？" })).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "清空历史" }));

    expect(
      await within(historyPage).findByText("还没有转换记录。完成一次转换后，任务会出现在这里。"),
    ).toBeInTheDocument();
  });

  it("历史详情弹窗展示方向、引擎与统计", async () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: /^选择文件$/ }));
    await screen.findByText("GB-T-28448.docx");
    fireEvent.click(screen.getByRole("button", { name: "开始转换" }));
    await screen.findByRole("heading", { name: "转换完成" }, { timeout: 3000 });

    fireEvent.click(screen.getByRole("button", { name: "转换历史" }));
    const historyPage = document.getElementById("page-history") as HTMLElement;
    const row = await within(historyPage).findByRole("button", { name: /GB-T-28448\.docx/ });
    fireEvent.click(row);

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByRole("heading", { name: "GB-T-28448.docx" })).toBeInTheDocument();
    expect(within(dialog).getByText("DOCX → Markdown")).toBeInTheDocument();
    expect(within(dialog).getByText(/anydoc/)).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "重跑" })).toBeInTheDocument();
  });
});
