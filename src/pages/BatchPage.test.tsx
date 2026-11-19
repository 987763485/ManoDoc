import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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

async function openBatch() {
  fireEvent.click(screen.getByRole("button", { name: "语料制作" }));
  return document.getElementById("page-batch") as HTMLElement;
}

describe("语料制作", () => {
  it("添加文件后显示队列并完成批次", async () => {
    setup();
    const page = await openBatch();

    fireEvent.click(within(page).getByRole("button", { name: "添加文件" }));
    expect(await within(page).findByText("GB-T-28448.docx")).toBeInTheDocument();
    expect(within(page).getByText("等级保护测评2.0要求指标清单v1.xlsx")).toBeInTheDocument();

    fireEvent.click(within(page).getAllByRole("button", { name: "开始批量" })[0]);
    expect(await screen.findByText("批次完成", {}, { timeout: 3000 })).toBeInTheDocument();

    await waitFor(() => {
      expect(within(page).getAllByText("完成")).toHaveLength(2);
    });
    const openPackage = within(page).getAllByRole("button", { name: "打开语料包目录" })[0];
    expect(openPackage).toBeEnabled();
  });

  it("高级设置默认折叠，展开后显示并发与清洗选项", async () => {
    setup();
    const page = await openBatch();

    const toggle = within(page).getByRole("button", { name: /高级设置/ });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(within(page).queryByText("清洗与语料")).not.toBeInTheDocument();

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(within(page).getByText("清洗与语料")).toBeInTheDocument();
    expect(within(page).getByRole("switch", { name: "切块" })).toBeInTheDocument();
  });

  it("输出格式锁定 Markdown，图片资产禁用", async () => {
    setup();
    const page = await openBatch();

    expect(within(page).getByText("批量输出固定为 Markdown；其他格式请在单文件转换中选择。")).toBeInTheDocument();
    fireEvent.click(within(page).getByRole("button", { name: /高级设置/ }));
    const imageSwitch = within(page).getByRole("switch", { name: "图片落 assets/" });
    expect(imageSwitch).toBeDisabled();
    expect(imageSwitch).toHaveAttribute("aria-checked", "false");
  });

  it("批量历史显示文件数与块数，并可重跑回填队列", async () => {
    setup();
    const page = await openBatch();
    fireEvent.click(within(page).getByRole("button", { name: "添加文件" }));
    await within(page).findByText("GB-T-28448.docx");
    fireEvent.click(within(page).getAllByRole("button", { name: "开始批量" })[0]);
    await screen.findByText("批次完成", {}, { timeout: 3000 });

    fireEvent.click(screen.getByRole("button", { name: "转换历史" }));
    const history = document.getElementById("page-history") as HTMLElement;
    expect(await within(history).findByText("corpus")).toBeInTheDocument();
    expect(within(history).getByText(/批量 · 2 文件/)).toBeInTheDocument();
    expect(within(history).getByText(/8 语料块/)).toBeInTheDocument();

    fireEvent.click(within(history).getByRole("button", { name: /corpus/ }));
    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "重跑" }));

    expect(await within(page).findByText("GB-T-28448.docx")).toBeInTheDocument();
    expect(within(page).getAllByRole("button", { name: "开始批量" })[0]).toBeEnabled();
  });
});
