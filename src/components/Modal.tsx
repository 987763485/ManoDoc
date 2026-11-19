import type { ReactNode } from "react";

import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";

/**
 * 全应用统一弹窗：shadcn/ui Dialog（Radix），固定屏幕居中。
 * 面板样式仍由调用方的 history-modal 等类提供。
 */
export function Modal({
  open,
  onClose,
  className = "history-dialog",
  labelledBy,
  children,
}: {
  open: boolean;
  onClose: () => void;
  className?: string;
  labelledBy?: string;
  children: ReactNode;
}) {
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className={className} aria-labelledby={labelledBy}>
        {/* Radix 需要 Title 才算可访问；可见标题由调用方的 h2 提供。 */}
        <DialogTitle className="sr-only">DullDoc</DialogTitle>
        {children}
      </DialogContent>
    </Dialog>
  );
}
