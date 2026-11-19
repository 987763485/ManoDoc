import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
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

describe("设置 · 引擎页", () => {
  it("显示 xberg 就绪与 OCR 模型状态", async () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: "系统设置" }));
    fireEvent.click(screen.getByRole("tab", { name: "引擎" }));

    expect(await screen.findByText("xberg")).toBeInTheDocument();
    expect(screen.getByText(/第二内核/)).toBeInTheDocument();
    expect(screen.getByText("PaddleOCR · PP-OCRv6 small")).toBeInTheDocument();
    // 浏览器 mock 的模型状态为已安装。
    expect(await screen.findByText("已安装")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "安装 OCR 模型" })).not.toBeInTheDocument();
  });
});
