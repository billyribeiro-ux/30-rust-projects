export type Bookmark = {
  id: string;
  url: string;
  title: string;
  description: string;
  created_at: string;
  updated_at: string;
  tags: string[];
};

export type TagWithCount = {
  name: string;
  count: number;
};

export type ApiError = {
  error: { code: string; message: string };
};
