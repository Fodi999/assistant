import { ActionButton } from '../../../components/admin/ActionButton';
import { AdminPanel } from '../../../components/admin/AdminPanel';
import type { IconsSiteContent } from '../../../api/iconsSite';
import type { ChurchArticle, ChurchIcon, ChurchImportPreview } from '../../../api/churchContent';
import { ContentRow } from './ChurchWorkflowSteps';
import { imageForContent } from './churchHelpers';

export function ChurchImportPanel({
  importPreview,
  legacyContent,
  unlinkedArticles,
  icons,
  importing,
  onPreviewImport,
  onApplyImport,
  onDeleteArticle,
  onClose
}: {
  importPreview: ChurchImportPreview | null;
  legacyContent: IconsSiteContent | null;
  unlinkedArticles: ChurchArticle[];
  icons: ChurchIcon[];
  importing: boolean;
  onPreviewImport: () => void;
  onApplyImport: () => void;
  onDeleteArticle: (article: ChurchArticle) => void;
  onClose: () => void;
}) {
  const legacyItems = legacyContent ? [
    ...legacyContent.icons.map((item) => ({ key: `icon-${item.id}`, title: item.title, meta: `${item.status} · ${item.calendarDate || 'без даты'} · икона` })),
    ...legacyContent.prayers.map((item) => ({ key: `prayer-${item.id}`, title: item.title, meta: `${item.status} · молитва` })),
    ...legacyContent.pages.map((item) => ({ key: `page-${item.id}`, title: item.title || item.h1, meta: `${item.status} · SEO-страница` }))
  ].filter((item) => item.title).slice(0, 8) : [];

  return (
    <AdminPanel
      title="Импорт старых публикаций"
      icon="save"
      meta={<ActionButton onClick={onClose}>Скрыть</ActionButton>}
    >
      {importPreview ? (
        <div className="church-import-preview">
          <span><strong>{importPreview.calendarDays}</strong> дней календаря</span>
          <span><strong>{importPreview.icons}</strong> икон</span>
          <span><strong>{importPreview.prayers}</strong> молитв</span>
          <span><strong>{importPreview.articles}</strong> статей</span>
          <span><strong>{importPreview.gospel}</strong> чтений</span>
          {importPreview.errors.length ? <small className="admin-form-error">{importPreview.errors.length} ошибок импорта</small> : null}
        </div>
      ) : null}

      {legacyContent ? (
        <div className="legacy-content-bridge">
          <div>
            <strong>Старые публикации сайта не удалены</strong>
            <span>Часть материалов на сайте всё ещё берётся из старого хранилища icons-site. Импортируйте их в новый редактор Church Content.</span>
          </div>
          <div className="admin-header-actions">
            <ActionButton onClick={onPreviewImport} disabled={importing}>Preview import</ActionButton>
            <ActionButton icon="save" tone="primary" onClick={onApplyImport} disabled={importing}>{importing ? 'Импортируем...' : 'Импортировать'}</ActionButton>
          </div>
        </div>
      ) : null}

      {legacyItems.length ? (
        <div className="legacy-content-list">
          {legacyItems.map((item) => (
            <span key={item.key}>
              <strong>{item.title}</strong>
              <small>{item.meta}</small>
            </span>
          ))}
        </div>
      ) : null}

      {unlinkedArticles.length ? (
        <div className="church-content-list-panel">
          <div className="church-content-list-head"><strong>Статьи без привязки к дню</strong></div>
          <div className="church-content-list">
            {unlinkedArticles.map((article) => (
              <ContentRow
                key={article.id}
                title={article.title}
                meta={`${article.language} · ${article.status} · /church/articles/${article.slug}`}
                body={article.seoDescription || article.content}
                image={imageForContent(article.iconId, [], icons)}
                onDelete={() => onDeleteArticle(article)}
              />
            ))}
          </div>
        </div>
      ) : null}

      {!importPreview && !legacyContent && !unlinkedArticles.length ? (
        <p className="admin-table-empty">Старых публикаций для импорта не найдено.</p>
      ) : null}
    </AdminPanel>
  );
}
