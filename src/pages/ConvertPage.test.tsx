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
  it("模型就绪时可选 xberg，并在成功后显示实际引擎", async () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: /^选择文件$/ }));
    await screen.findByText("GB-T-28448.docx");

    const engine = document.querySelector(".route-row select") as HTMLSelectElement;
    expect(engine).not.toBeNull();
    const xberg = Array.from(engine.options).find((option) => option.value === "xberg");
    expect(xberg).toBeDefined();
    // 浏览器 mock 返回 ocrReady=true，因此 xberg 不应被禁用。
    await waitFor(() => expect(xberg?.disabled).toBe(false));

    fireEvent.change(engine, { target: { value: "xberg" } });
    expect(engine.value).toBe("xberg");

    fireEvent.click(screen.getByRole("button", { name: "开始转换" }));
    expect(
      await screen.findByRole("heading", { name: "转换完成" }, { timeout: 3000 }),
    ).toBeInTheDocument();
    expect(screen.getByText(/实际引擎/)).toHaveTextContent("anydoc");
  });
});
