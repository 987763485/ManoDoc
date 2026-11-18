const iconMap: Record<string, string> = {
  DOC: "WORD",
  DOCX: "WORD",
  ODT: "ODT",
  RTF: "RTF",
  EPUB: "EPUB",
  PDF: "PDF",
  PPT: "PPT",
  PPTX: "PPT",
  ODP: "ODP",
  XLS: "ECEL",
  XLSX: "ECEL",
  ODS: "ODS",
  CSV: "CSV",
  MD: "MD",
  HTML: "HTML",
  TXT: "TXT",
  PNG: "PNG",
  JPG: "JPG",
  JPEG: "JPG",
};

/** 文件类型图标与方向矩阵共用同一套命名，缺失时回退综合文档。 */
export function FileIcon({
  format,
  name,
  size = 46,
}: {
  format?: string | null;
  name?: string;
  size?: number;
}) {
  const ext = (format ?? name?.split(".").pop() ?? "").toUpperCase();
  const icon = iconMap[ext] ?? "综合文档";
  return (
    <span className="file-icon" style={{ width: size, height: size, flexBasis: size }}>
      <img src={`/icons/${icon}.svg`} alt="" />
    </span>
  );
}
