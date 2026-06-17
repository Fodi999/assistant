import { useEffect, useMemo, useState } from 'react';
import { ExternalLink, Plus, RefreshCw, Save, Trash2 } from 'lucide-react';
import {
  almabuildSiteUrl,
  getAlmabuildContent,
  saveAlmabuildContent,
  type AlmabuildContent,
  type Kit,
  type MaterialCategory,
  type Product,
  type Project
} from '../../api/almabuild';
import type { AlmabuildSection } from '../../types/admin';

type AlmabuildLanguage = 'ru' | 'kk' | 'en';
type LocalizedStringKey = 'title' | 'text' | 'category' | 'spec' | 'meta';
type LocalizedListKey = 'bullets' | 'items';

const almabuildLanguages: Array<{ key: AlmabuildLanguage; label: string; name: string }> = [
  { key: 'ru', label: 'RU', name: 'Русский' },
  { key: 'kk', label: 'KZ', name: 'Қазақша' },
  { key: 'en', label: 'EN', name: 'English' }
];

const emptyContent: AlmabuildContent = {
  materialCategories: [],
  products: [],
  kits: [],
  projects: []
};

function splitList(value: string) {
  return value.split('\n').map((item) => item.trim()).filter(Boolean);
}

function joinList(value: string[]) {
  return value.join('\n');
}

function langSuffix(lang: AlmabuildLanguage) {
  if (lang === 'kk') return 'Kk';
  if (lang === 'en') return 'En';
  return 'Ru';
}

function localizedString<T extends Record<string, unknown>>(item: T, key: LocalizedStringKey, lang: AlmabuildLanguage): string {
  const localized = item[`${key}${langSuffix(lang)}`] as string | undefined;
  if (localized) return localized;
  return lang === 'ru' ? String(item[key] || '') : '';
}

function localizedList<T extends Record<string, unknown>>(item: T, key: LocalizedListKey, lang: AlmabuildLanguage): string[] {
  const localized = item[`${key}${langSuffix(lang)}`] as string[] | undefined;
  if (Array.isArray(localized) && localized.length) return localized;
  return lang === 'ru' && Array.isArray(item[key]) ? item[key] as string[] : [];
}

function patchLocalizedString<T extends Record<string, unknown>>(item: T, key: LocalizedStringKey, lang: AlmabuildLanguage, value: string): Partial<T> {
  const patch = { [`${key}${langSuffix(lang)}`]: value } as Partial<T>;
  if (lang === 'ru') return { ...patch, [key]: value } as Partial<T>;
  return patch;
}

function patchLocalizedList<T extends Record<string, unknown>>(item: T, key: LocalizedListKey, lang: AlmabuildLanguage, value: string[]): Partial<T> {
  const patch = { [`${key}${langSuffix(lang)}`]: value } as Partial<T>;
  if (lang === 'ru') return { ...patch, [key]: value } as Partial<T>;
  return patch;
}

function categoryTemplate(index: number): MaterialCategory {
  return {
    index: '',
    slug: '',
    title: '',
    text: '',
    bullets: [],
    photo: ''
  };
}

function productTemplate(categorySlug = ''): Product {
  return {
    categorySlug,
    category: '',
    title: '',
    spec: '',
    photo: ''
  };
}

function kitTemplate(): Kit {
  return {
    title: '',
    text: '',
    items: []
  };
}

function projectTemplate(): Project {
  return {
    title: '',
    meta: '',
    photo: ''
  };
}

function LanguageTabs({ active, onChange }: { active: AlmabuildLanguage; onChange: (lang: AlmabuildLanguage) => void }) {
  return (
    <div className="almabuild-language-tabs" aria-label="Язык редактирования">
      <span>Язык редактирования</span>
      <div>
        {almabuildLanguages.map((lang) => (
          <button key={lang.key} className={active === lang.key ? 'active' : ''} type="button" onClick={() => onChange(lang.key)}>
            {lang.label}
            <small>{lang.name}</small>
          </button>
        ))}
      </div>
    </div>
  );
}

function Field({
  label,
  help,
  children,
  wide = false
}: {
  label: string;
  help: string;
  children: React.ReactNode;
  wide?: boolean;
}) {
  return (
    <label className={'almabuild-field' + (wide ? ' wide' : '')}>
      <span>{label}</span>
      {children}
      <small>{help}</small>
    </label>
  );
}

function StaticSectionNotice({ title, text }: { title: string; text: string }) {
  return (
    <article className="almabuild-panel">
      <div className="section-head">
        <div>
          <span className="eyebrow">Раздел сайта</span>
          <h2>{title}</h2>
          <p className="section-note">{text}</p>
        </div>
      </div>
      <div className="site-preview">
        <span>Следующий шаг</span>
        <strong>Подключить этот блок к backend-контенту</strong>
        <p>Сейчас этот раздел хранится в коде публичного сайта Kazaxbud. Материалы, каталог, проекты и смета уже редактируются из CMS.</p>
      </div>
    </article>
  );
}

export function AlmabuildPage({ activeSection }: { activeSection: AlmabuildSection }) {
  const [content, setContent] = useState<AlmabuildContent>(emptyContent);
  const [activeLang, setActiveLang] = useState<AlmabuildLanguage>('ru');
  const [projectEditorOpen, setProjectEditorOpen] = useState(false);
  const [projectDraft, setProjectDraft] = useState<Project>(() => projectTemplate());
  const [editingProjectIndex, setEditingProjectIndex] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const stats = useMemo(() => [
    { label: 'Категории', value: String(content.materialCategories.length), note: 'Блок «Материалы» + фильтр каталога' },
    { label: 'Товары', value: String(content.products.length), note: 'Карточки и страница каталога' },
    { label: 'Комплекты', value: String(content.kits.length), note: 'Блок готовых наборов' },
    { label: 'Проекты', value: String(content.projects.length), note: 'Кейсы на главной' }
  ], [content]);

  async function loadContent() {
    setLoading(true);
    setError(null);
    setMessage(null);
    try {
      setContent(await getAlmabuildContent());
      setMessage('Контент ALMABUILD загружен из backend');
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : 'Не удалось загрузить ALMABUILD');
    } finally {
      setLoading(false);
    }
  }

  async function publishContent() {
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      setContent(await saveAlmabuildContent(content));
      setMessage('Опубликовано. Сайт kazaxbud читает эти данные через backend.');
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : 'Не удалось сохранить ALMABUILD');
    } finally {
      setSaving(false);
    }
  }

  useEffect(() => {
    void loadContent();
  }, []);

  useEffect(() => {
    if (activeSection !== 'projects') setProjectEditorOpen(false);
  }, [activeSection]);

  function updateCategory(index: number, patch: Partial<MaterialCategory>) {
    setContent((current) => ({
      ...current,
      materialCategories: current.materialCategories.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item)
    }));
  }

  function updateProduct(index: number, patch: Partial<Product>) {
    setContent((current) => ({
      ...current,
      products: current.products.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item)
    }));
  }

  function updateKit(index: number, patch: Partial<Kit>) {
    setContent((current) => ({
      ...current,
      kits: current.kits.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item)
    }));
  }

  function openNewProject() {
    setProjectDraft(projectTemplate());
    setEditingProjectIndex(null);
    setProjectEditorOpen(true);
    setMessage(null);
  }

  function openEditProject(index: number) {
    setProjectDraft({ ...content.projects[index] });
    setEditingProjectIndex(index);
    setProjectEditorOpen(true);
    setMessage(null);
  }

  function patchProjectDraft(patch: Partial<Project>) {
    setProjectDraft((current) => ({ ...current, ...patch }));
  }

  function saveProjectDraft() {
    setContent((current) => {
      if (editingProjectIndex === null) {
        return { ...current, projects: [projectDraft, ...current.projects] };
      }
      return {
        ...current,
        projects: current.projects.map((item, index) => index === editingProjectIndex ? projectDraft : item)
      };
    });
    setProjectEditorOpen(false);
    setMessage('Проект сохранен в черновике CMS. Нажми «Опубликовать», чтобы отправить изменения в backend.');
  }

  function deleteProjectDraft() {
    if (editingProjectIndex === null) {
      setProjectEditorOpen(false);
      return;
    }
    setContent((current) => ({
      ...current,
      projects: current.projects.filter((_, index) => index !== editingProjectIndex)
    }));
    setProjectEditorOpen(false);
    setMessage('Проект удален из черновика CMS. Нажми «Опубликовать», чтобы сохранить удаление в backend.');
  }

  return (
    <section className="almabuild-page">
      <header className="almabuild-hero" id="almabuild-overview">
        <div>
          <span className="eyebrow">ALMABUILD PRO CMS</span>
          <h2>Управление сайтом kazaxbud</h2>
          <p>Каждый блок ниже подписан так же, как он выглядит на сайте. Изменения сохраняются в backend и появляются на публичных страницах после публикации.</p>
        </div>
        <div className="almabuild-actions">
          <a className="btn btn-quiet" href={almabuildSiteUrl} target="_blank" rel="noreferrer"><ExternalLink size={16} />Открыть сайт</a>
          <button className="btn btn-quiet" type="button" onClick={loadContent} disabled={loading || saving}><RefreshCw size={16} />Обновить</button>
          <button className="btn btn-primary" type="button" onClick={publishContent} disabled={loading || saving}><Save size={16} />{saving ? 'Публикуем...' : 'Опубликовать'}</button>
        </div>
      </header>

      {message && <p className="almabuild-alert">{message}</p>}
      {error && <p className="almabuild-alert error">{error}</p>}

      <LanguageTabs active={activeLang} onChange={setActiveLang} />

      <div className="almabuild-section-workspace">
          <div className="metrics-grid">
            {stats.map((item) => (
              <article className="metric-card" key={item.label}>
                <span className="metric-label">{item.label}</span>
                <strong className="metric-value">{item.value}</strong>
                <p className="metric-note">{item.note}</p>
              </article>
            ))}
          </div>

      {activeSection === 'services' ? (
        <StaticSectionNotice title="Услуги" text="Верхний раздел сайта: услуги, подход и оффер. Пока это статический блок публичного сайта, вынесли его в боковую панель, чтобы структура редактирования совпадала с сайтом." />
      ) : null}

      {activeSection === 'materials' ? <article className="almabuild-panel" id="almabuild-categories">
        <div className="section-head">
          <div>
            <span className="eyebrow">Блок сайта: «Материалы для коммерческой отделки»</span>
            <h2>Категории материалов</h2>
            <p className="section-note">Эти карточки видны на главной в блоке «Материалы» и как фильтры на странице каталога.</p>
          </div>
          <button className="btn btn-quiet" type="button" onClick={() => setContent((current) => ({ ...current, materialCategories: [...current.materialCategories, categoryTemplate(current.materialCategories.length)] }))}><Plus size={16} />Добавить категорию</button>
        </div>
        <div className="almabuild-card-list">
          {content.materialCategories.map((category, index) => (
            <article className="almabuild-edit-card" key={category.slug + '-' + index}>
              <div className="edit-card-head">
                <div>
                  <span>Категория на сайте #{index + 1}</span>
                  <h3>{localizedString(category, 'title', activeLang) || category.title || 'Без названия'}</h3>
                </div>
                <button className="icon-danger" type="button" aria-label="Удалить категорию" onClick={() => setContent((current) => ({ ...current, materialCategories: current.materialCategories.filter((_, itemIndex) => itemIndex !== index) }))}><Trash2 size={17} /></button>
              </div>
              <div className="almabuild-form-grid categories">
                <Field label={`Название карточки ${activeLang.toUpperCase()}`} help="Крупный заголовок в блоке «Материалы» и название фильтра в каталоге."><input value={localizedString(category, 'title', activeLang)} placeholder={category.title} onChange={(event) => updateCategory(index, patchLocalizedString(category, 'title', activeLang, event.target.value))} /></Field>
                <Field label="URL slug" help="Адрес категории: /catalog/slug. Лучше латиница без пробелов."><input value={category.slug} onChange={(event) => updateCategory(index, { slug: event.target.value })} /></Field>
                <Field label="Номер" help="Маленький индекс на карточке, например [0:1]."><input value={category.index} onChange={(event) => updateCategory(index, { index: event.target.value })} /></Field>
                <Field label="Визуальный класс" help="Технический ключ фоновой картинки/стиля."><input value={category.photo} onChange={(event) => updateCategory(index, { photo: event.target.value })} /></Field>
                <Field label={`Описание карточки ${activeLang.toUpperCase()}`} help="Текст под названием категории на главной." wide><textarea value={localizedString(category, 'text', activeLang)} placeholder={category.text} onChange={(event) => updateCategory(index, patchLocalizedString(category, 'text', activeLang, event.target.value))} /></Field>
                <Field label={`Список внутри категории ${activeLang.toUpperCase()}`} help="Показывается в деталях/админке: каждый пункт с новой строки." wide><textarea value={joinList(localizedList(category, 'bullets', activeLang))} placeholder={joinList(category.bullets)} onChange={(event) => updateCategory(index, patchLocalizedList(category, 'bullets', activeLang, splitList(event.target.value)))} /></Field>
              </div>
              <div className="site-preview">
                <span>Как это читается на сайте</span>
                <strong>{category.index} · {localizedString(category, 'title', activeLang) || category.title}</strong>
                <p>{localizedString(category, 'text', activeLang) || category.text}</p>
              </div>
            </article>
          ))}
        </div>
      </article> : null}

      {activeSection === 'catalog' ? <article className="almabuild-panel" id="almabuild-products">
        <div className="section-head">
          <div>
            <span className="eyebrow">Блок сайта: «Сопутствующие товары» + страница каталога</span>
            <h2>Товары и материалы</h2>
            <p className="section-note">Эти позиции отображаются карточками на главной и в каталоге. Категория связывает товар с фильтром.</p>
          </div>
          <button className="btn btn-quiet" type="button" onClick={() => setContent((current) => ({ ...current, products: [productTemplate(current.materialCategories[0]?.slug), ...current.products] }))}><Plus size={16} />Добавить товар</button>
        </div>
        <div className="almabuild-card-list compact">
          {content.products.map((product, index) => (
            <article className="almabuild-edit-card product" key={product.title + '-' + index}>
              <div className="edit-card-head">
                <div><span>Карточка товара #{index + 1}</span><h3>{localizedString(product, 'title', activeLang) || product.title || 'Без названия'}</h3></div>
                <button className="icon-danger" type="button" aria-label="Удалить товар" onClick={() => setContent((current) => ({ ...current, products: current.products.filter((_, itemIndex) => itemIndex !== index) }))}><Trash2 size={17} /></button>
              </div>
              <div className="almabuild-form-grid products">
                <Field label={`Название товара ${activeLang.toUpperCase()}`} help="Главный текст на карточке товара."><input value={localizedString(product, 'title', activeLang)} placeholder={product.title} onChange={(event) => updateProduct(index, patchLocalizedString(product, 'title', activeLang, event.target.value))} /></Field>
                <Field label={`Метка категории ${activeLang.toUpperCase()}`} help="Короткая подпись над названием: ГКЛ, Профили, Свет."><input value={localizedString(product, 'category', activeLang)} placeholder={product.category} onChange={(event) => updateProduct(index, patchLocalizedString(product, 'category', activeLang, event.target.value))} /></Field>
                <Field label="Раздел каталога" help="Определяет, в каком фильтре каталога появится товар."><select value={product.categorySlug} onChange={(event) => updateProduct(index, { categorySlug: event.target.value })}>{content.materialCategories.map((category) => <option key={category.slug} value={category.slug}>{category.title}</option>)}</select></Field>
                <Field label={`Характеристики ${activeLang.toUpperCase()}`} help="Краткий размер, класс или назначение."><input value={localizedString(product, 'spec', activeLang)} placeholder={product.spec} onChange={(event) => updateProduct(index, patchLocalizedString(product, 'spec', activeLang, event.target.value))} /></Field>
                <Field label="Визуальный класс" help="Технический ключ оформления карточки."><input value={product.photo} onChange={(event) => updateProduct(index, { photo: event.target.value })} /></Field>
              </div>
              <div className="site-preview small"><span>Карточка</span><strong>{localizedString(product, 'category', activeLang) || product.category}</strong><p>{localizedString(product, 'title', activeLang) || product.title} · {localizedString(product, 'spec', activeLang) || product.spec}</p></div>
            </article>
          ))}
        </div>
      </article> : null}

      {activeSection === 'estimate' ? (
        <article className="almabuild-panel" id="almabuild-kits">
          <div className="section-head">
            <div>
              <span className="eyebrow">Блок сайта: «Готовые наборы под объект»</span>
              <h2>Комплекты</h2>
              <p className="section-note">Наборы материалов, которые можно быстро добавить в смету.</p>
            </div>
            <button className="btn btn-quiet" type="button" onClick={() => setContent((current) => ({ ...current, kits: [kitTemplate(), ...current.kits] }))}><Plus size={16} />Добавить</button>
          </div>
          <div className="almabuild-card-list">
            {content.kits.map((kit, index) => (
              <article className="almabuild-edit-card" key={kit.title + '-' + index}>
                <div className="edit-card-head"><div><span>Комплект #{index + 1}</span><h3>{localizedString(kit, 'title', activeLang) || kit.title || 'Без названия'}</h3></div><button className="icon-danger" type="button" aria-label="Удалить комплект" onClick={() => setContent((current) => ({ ...current, kits: current.kits.filter((_, itemIndex) => itemIndex !== index) }))}><Trash2 size={17} /></button></div>
                <div className="almabuild-form-grid one">
                  <Field label={`Название комплекта ${activeLang.toUpperCase()}`} help="Заголовок карточки в блоке комплектов."><input value={localizedString(kit, 'title', activeLang)} placeholder={kit.title} onChange={(event) => updateKit(index, patchLocalizedString(kit, 'title', activeLang, event.target.value))} /></Field>
                  <Field label={`Описание ${activeLang.toUpperCase()}`} help="Одна строка под названием комплекта."><textarea value={localizedString(kit, 'text', activeLang)} placeholder={kit.text} onChange={(event) => updateKit(index, patchLocalizedString(kit, 'text', activeLang, event.target.value))} /></Field>
                  <Field label={`Состав комплекта ${activeLang.toUpperCase()}`} help="Каждый пункт с новой строки, показывается списком."><textarea value={joinList(localizedList(kit, 'items', activeLang))} placeholder={joinList(kit.items)} onChange={(event) => updateKit(index, patchLocalizedList(kit, 'items', activeLang, splitList(event.target.value)))} /></Field>
                </div>
              </article>
            ))}
          </div>
        </article>
      ) : null}

      {activeSection === 'projects' ? (
        <article className="almabuild-panel" id="almabuild-projects">
          <div className="section-head">
            <div>
              <span className="eyebrow">Блок сайта: «Коммерческие пространства»</span>
              <h2>Проекты</h2>
              <p className="section-note">Кейсы на главной: название объекта, формат, площадь и сроки. Редактирование открывается отдельно, как в блогах кулинарного сайта.</p>
            </div>
            <button className="btn btn-primary" type="button" onClick={openNewProject}><Plus size={16} />Новый проект</button>
          </div>
          <section className="ops-panel almabuild-project-list">
            <table className="ops-table">
              <thead>
                <tr><th>Проект</th><th>Описание</th><th>Визуал</th><th>Языки</th><th /></tr>
              </thead>
              <tbody>
                {content.projects.map((project, index) => (
                  <tr key={`${project.title}-${index}`}>
                    <td><strong>{localizedString(project, 'title', activeLang) || project.title || 'Без названия'}</strong><small>Проект #{index + 1}</small></td>
                    <td>{localizedString(project, 'meta', activeLang) || project.meta || 'Нет описания'}</td>
                    <td><code>{project.photo || 'photo-project'}</code></td>
                    <td>{['ru', 'kk', 'en'].filter((lang) => localizedString(project, 'title', lang as AlmabuildLanguage)).map((lang) => lang.toUpperCase()).join(' / ') || 'RU'}</td>
                    <td><button className="table-action" type="button" onClick={() => openEditProject(index)}>Редактировать</button></td>
                  </tr>
                ))}
                {!content.projects.length ? <tr><td colSpan={5}>Проектов пока нет. Нажми «Новый проект», чтобы добавить первый кейс.</td></tr> : null}
              </tbody>
            </table>
          </section>
        </article>
      ) : null}

      {activeSection === 'contact' ? (
        <StaticSectionNotice title="Контакты" text="Форма заявки уже отправляет лиды в backend. Тексты контактов и телефон пока статические на публичном сайте; теперь у раздела есть отдельное место в редакторе." />
      ) : null}
      </div>

      {projectEditorOpen ? (
        <div className="modal-overlay">
          <div className="editor-modal almabuild-project-modal">
            <div className="editor-modal-head">
              <div>
                <p className="eyebrow">{editingProjectIndex === null ? 'Новый проект' : 'Редактирование проекта'}</p>
                <h2>{localizedString(projectDraft, 'title', activeLang) || projectDraft.title || 'Коммерческий проект'}</h2>
              </div>
              <div className="editor-actions">
                <button className="btn btn-quiet" type="button" onClick={() => setProjectEditorOpen(false)}>Закрыть</button>
                <button className="btn btn-primary" type="button" onClick={saveProjectDraft}>Сохранить</button>
              </div>
            </div>

            <div className="analytics-mode-switcher content-lang-tabs">
              {almabuildLanguages.map((language) => (
                <button key={language.key} className={activeLang === language.key ? 'analytics-mode-button active' : 'analytics-mode-button'} type="button" onClick={() => setActiveLang(language.key)}>
                  {language.label}
                </button>
              ))}
            </div>

            <div className="editor-grid">
              <label className="editor-field">
                <span>Название проекта {activeLang.toUpperCase()}</span>
                <input value={localizedString(projectDraft, 'title', activeLang)} placeholder={projectDraft.title || 'BUTIK KZ'} onChange={(event) => patchProjectDraft(patchLocalizedString(projectDraft, 'title', activeLang, event.target.value))} />
              </label>
              <label className="editor-field">
                <span>Визуальный класс</span>
                <input value={projectDraft.photo} onChange={(event) => patchProjectDraft({ photo: event.target.value })} placeholder="photo-retail" />
              </label>
            </div>

            <label className="editor-field">
              <span>Описание проекта {activeLang.toUpperCase()}</span>
              <textarea value={localizedString(projectDraft, 'meta', activeLang)} placeholder={projectDraft.meta || 'Магазин одежды · 320 м² · 28 дней'} onChange={(event) => patchProjectDraft(patchLocalizedString(projectDraft, 'meta', activeLang, event.target.value))} />
            </label>

            <div className="site-preview">
              <span>Как это читается на сайте</span>
              <strong>{localizedString(projectDraft, 'title', activeLang) || projectDraft.title || 'Название проекта'}</strong>
              <p>{localizedString(projectDraft, 'meta', activeLang) || projectDraft.meta || 'Формат · площадь · срок'}</p>
            </div>

            <div className="editor-actions">
              <button className="btn btn-danger" type="button" onClick={deleteProjectDraft}>{editingProjectIndex === null ? 'Отменить' : 'Удалить'}</button>
              <button className="btn btn-primary" type="button" onClick={saveProjectDraft}>Сохранить</button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
