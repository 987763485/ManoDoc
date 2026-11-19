import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import App from "../App";

function setup() {
  render(<App />);
}

beforeEach(() => {
  localStorage.clear();
  document.documentElement.dataset.theme = "light";
});

afterEach(() => {
  cleanup();
});

describe("文档转换的引擎选择", () => {
  it("Markdown 输入显示导出目标与质量提示", async () => {
    setup();
    const drop = document.getElementById("file-empty") as HTMLElement;
    const file = new File(["# 标题\n\n正文"], "note.md", { type: "text/markdown" });
    Object.defineProperty(file, "path", { value: "/tmp/note.md" });
    fireEvent.drop(drop, { dataTransfer: { files: [file] } });

    expect(await screen.findByText("note.md")).toBeInTheDocument();
    const target = document.getElementById("target-format") as HTMLElement;
    expect(target).toHaveTextContent("Word");

    fireEvent.pointerDown(target, { button: 0, pointerType: "mouse" });
    const targetValues = (await screen.findAllByRole("option")).map((option) => option.textContent);
    expect(targetValues).toEqual(["Word", "PDF", "HTML", "EPUB"]);

    fireEvent.click(screen.getByRole("option", { name: "PDF" }));
    expect(screen.getByText(/由 pandoc 生成 Typst 后编译/)).toBeInTheDocument();

    fireEvent.pointerDown(target, { button: 0, pointerType: "mouse" });
    fireEvent.click(screen.getByRole("option", { name: "Word" }));
    expect(screen.queryByText(/由 pandoc 生成 Typst 后编译/)).not.toBeInTheDocument();
  });

  it("模型就绪时可选 xberg，并在成功后显示实际引擎", async () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: /^选择文件$/ }));
    await screen.findByText("GB-T-28448.docx");

    const engine = document.querySelector('.route-row .ui-select-trigger[aria-label="引擎"]') as HTMLElement;
    fireEvent.pointerDown(engine, { button: 0, pointerType: "mouse" });
    const xberg = Array.from(document.querySelectorAll<HTMLElement>('[role="option"]')).find(
      (option) => option.textContent?.includes("xberg"),
    );
    expect(xberg).toBeDefined();
    // 浏览器 mock 返回 ocrReady=true，因此 xberg 不应被禁用。
    await waitFor(() => expect(xberg).not.toHaveAttribute("data-disabled"));
    fireEvent.click(xberg!);
    expect(engine).toHaveTextContent(/xberg/);

    fireEvent.click(screen.getByRole("button", { name: "开始转换" }));
    expect(
      await screen.findByRole("heading", { name: "转换完成" }, { timeout: 3000 }),
    ).toBeInTheDocument();
    expect(screen.getByText(/实际引擎/)).toHaveTextContent("anydoc");
  });
});
