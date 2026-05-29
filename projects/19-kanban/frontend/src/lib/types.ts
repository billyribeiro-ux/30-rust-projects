export type User = {
  id: string;
  email: string;
  name: string;
};

export type Role = 'viewer' | 'editor' | 'admin';

export type BoardSummary = {
  id: string;
  owner_id: string;
  name: string;
  slug: string;
  role: Role;
  created_at: string;
  updated_at: string;
};

export type ListRow = {
  id: string;
  board_id: string;
  name: string;
  position: number;
};

export type CardRow = {
  id: string;
  list_id: string;
  title: string;
  body: string;
  position: number;
  due_at: string | null;
  created_at: string;
  updated_at: string;
};

export type BoardFull = {
  id: string;
  owner_id: string;
  name: string;
  slug: string;
  role: Role;
  lists: ListRow[];
  cards: CardRow[];
};

export type MemberRow = {
  user_id: string;
  email: string;
  name: string;
  role: Role;
};

export type Comment = {
  id: string;
  card_id: string;
  author_id: string;
  author_email: string;
  author_name: string;
  body: string;
  created_at: string;
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
