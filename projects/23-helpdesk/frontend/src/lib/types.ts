export type User = { id: string; email: string; name: string };
export type Tenant = {
  id: string;
  slug: string;
  name: string;
  role: 'admin' | 'agent' | 'customer';
  sla_first_response_minutes: number;
  sla_resolve_minutes: number;
  created_at: string;
};
export type Ticket = {
  id: string;
  subject: string;
  status: 'open' | 'pending' | 'resolved' | 'closed';
  priority: 'low' | 'normal' | 'high' | 'urgent';
  assignee_user_id: string | null;
  created_at: string;
  updated_at: string;
};
export type Message = {
  id: string;
  author_user_id: string;
  body: string;
  internal: boolean;
  created_at: string;
};
export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
