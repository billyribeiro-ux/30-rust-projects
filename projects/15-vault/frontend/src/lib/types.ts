export type User = {
  id: string;
  email: string;
  name: string;
};

export type Folder = {
  id: string;
  parent_id: string | null;
  name: string;
  created_at: string;
  updated_at: string;
};

export type FileRow = {
  id: string;
  folder_id: string | null;
  name: string;
  size: number;
  mime: string;
  sha256: string;
  created_at: string;
  updated_at: string;
};

export type UploadCreated = { upload_id: string };

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
