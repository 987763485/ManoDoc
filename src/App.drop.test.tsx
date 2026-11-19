import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";

// 捕获 App 注册的全局拖入回调，用来验证任意页面的路由规则。
const drop = vi.hoisted(() => ({
  handler: null as null | ((paths: string[]) => void),
}));

vi.mock("./lib/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/ipc")>();
  return {
    ...actual,
    onFileDrop: async (handler: (paths: string[]) => void) => {
      drop.handler = handler;
      return () => {};
    },
    pathKind: async (path: string) =>
      path.endsWith("/") ? "directory" : "file",
  };
});

import App from "./App";

beforeEach(() => {
  localStorage.clear();
  document.documentElement.dataset.theme = "light";
  drop.handler = null;
});

afterEach(() => {
  cleanup();
});

async function handler() {
  await waitFor(() => expect(drop.handler).not.toBeNull());
  return drop.handler!;
}

describe("全局拖入路由", () => {
  it("单个文件拖入进入转换页并载入文件", async () => {
    render(<App />);
    const onDrop = await handler();
    await act(async () => {
      onDrop(["/tmp/报告.docx"]);
    });
    expect(await screen.findByText("报告.docx")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "开始转换" })).toBeInTheDocument();
  });

  it("多个文件拖入进入语料制作页", async () => {
    render(<App />);
    const onDrop = await handler();
    await act(async () => {
      onDrop(["/tmp/a.docx", "/tmp/b.pdf"]);
    });
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "语料制作" })).toBeInTheDocument(),
    );
  });

  it("文件夹拖入进入语料制作页", async () => {
    render(<App />);
    const onDrop = await handler();
    await act(async () => {
      onDrop(["/tmp/资料/"]);
    });
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "语料制作" })).toBeInTheDocument(),
    );
  });
});
