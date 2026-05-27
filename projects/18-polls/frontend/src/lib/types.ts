export type User = {
  id: string;
  email: string;
  name: string;
  email_verified: boolean;
};

export type OptionPublic = {
  id: string;
  label: string;
  position: number;
  count: number;
};

export type Poll = {
  id: string;
  slug: string;
  question: string;
  is_open: boolean;
  options: OptionPublic[];
  total_votes: number;
  created_at: string;
  closed_at: string | null;
};

export type PollSummary = {
  id: string;
  slug: string;
  question: string;
  is_open: boolean;
  option_count: number;
  vote_count: number;
  created_at: string;
};

/** Payload pushed by the SSE `counts` event. */
export type CountsEvent = {
  slug: string;
  total: number;
  counts: { option_id: string; label: string; position: number; count: number }[];
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
