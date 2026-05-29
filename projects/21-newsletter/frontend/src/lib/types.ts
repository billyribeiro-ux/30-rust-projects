export type User = { id: string; email: string; name: string };

export type PostSummary = {
  id: string;
  slug: string;
  title: string;
  summary: string;
  is_gated: boolean;
  published_at: string | null;
};

export type PostFull = PostSummary & {
  body_html: string;
  paywalled: boolean;
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
