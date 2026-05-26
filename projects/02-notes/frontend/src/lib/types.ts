export type NoteSummary = {
  id: string;
  slug: string;
  title: string;
  excerpt: string;
  updated_at: string;
};

export type Note = {
  id: string;
  slug: string;
  title: string;
  body_md: string;
  body_html: string;
  excerpt: string;
  created_at: string;
  updated_at: string;
};

export type ApiError = {
  error: {
    code: string;
    message: string;
  };
};
