export type User = { id: string; email: string; name: string };
export type ArticleSummary = {
  id: string;
  slug: string;
  title: string;
  summary: string;
  category_slug: string | null;
  locale: string;
  published_at: string | null;
};
export type Faq = { q: string; a: string };
export type ArticleFull = ArticleSummary & {
  body_html: string;
  faqs: Faq[];
  updated_at: string;
};
export type SearchHit = {
  id: string;
  slug: string;
  title: string;
  snippet: string;
  score: number;
};
export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
