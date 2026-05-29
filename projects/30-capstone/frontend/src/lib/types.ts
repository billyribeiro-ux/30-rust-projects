export type User = { id: string; email: string; name: string };
export type Tenant = {
  id: string;
  slug: string;
  name: string;
  role: 'owner' | 'admin' | 'member' | 'viewer';
  plan: 'free' | 'pro';
  status: string;
  created_at: string;
};
export type Project = { id: string; slug: string; name: string; created_at: string };
export type Task = {
  id: string;
  title: string;
  body: string;
  status: 'open' | 'in_progress' | 'done' | 'canceled';
  assignee_id: string | null;
  position: number;
  due_at: string | null;
  created_at: string;
  updated_at: string;
};
export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
