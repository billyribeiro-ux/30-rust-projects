export type BookStatus = 'want_to_read' | 'reading' | 'finished';

export type Book = {
  id: string;
  isbn: string | null;
  title: string;
  author: string;
  cover_url: string | null;
  pages: number | null;
  status: BookStatus;
  current_page: number;
  started_at: string | null;
  finished_at: string | null;
  created_at: string;
  updated_at: string;
};

export type Session = {
  id: string;
  book_id: string;
  pages_read: number;
  duration_minutes: number;
  occurred_at: string;
};

export type Highlight = {
  id: string;
  book_id: string;
  quote: string;
  note: string;
  page: number | null;
  created_at: string;
};

export type BookLookup = {
  isbn: string;
  title: string;
  author: string;
  cover_url: string | null;
  pages: number | null;
};

export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };

export const STATUS_LABEL: Record<BookStatus, string> = {
  want_to_read: 'Want to read',
  reading: 'Reading',
  finished: 'Finished'
};
