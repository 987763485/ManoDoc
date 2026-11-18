import { useI18n } from "../i18n";

/** 语料制作在纵向切片中只保留导航和清晰的后续接入说明，不伪造真实队列。 */
export function BatchPage({ active }: { active: boolean }) {
  const { t } = useI18n();
  return (
    <section className={`page${active ? " active" : ""}`} id="page-batch">
      <div className="page-head">
        <h1>{t("batch.title")}</h1>
        <p>{t("batch.sub")}</p>
        <div className="actions">
          <button className="btn secondary small" disabled>{t("batch.add")}</button>
          <button className="btn secondary small" disabled>{t("batch.addFolder")}</button>
          <button className="btn primary small" disabled>{t("batch.start")}</button>
        </div>
      </div>
      <div className="panel batch-placeholder">
        <span className="badge blue">{t("batch.later")}</span>
        <h2>{t("batch.placeholderTitle")}</h2>
        <p>{t("batch.placeholderBody")}</p>
      </div>
    </section>
  );
}
