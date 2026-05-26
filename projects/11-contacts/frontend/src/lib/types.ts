export type User = {
  id: string;
  email: string;
  name: string;
  email_verified: boolean;
};

export type Contact = {
  id: string;
  name: string;
  email: string;
  phone: string;
  company: string;
  notes: string;
  last_contacted_at: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  tags: string[];
};

export type DashboardData = {
  total_contacts: number;
  deleted_contacts: number;
  stale_contacts_7d: number;
  interactions_this_week: number;
  due_reminders: {
    id: string;
    contact_id: string;
    contact_name: string;
    body: string;
    due_at: string;
  }[];
  recent_contacts: {
    id: string;
    name: string;
    company: string;
    last_contacted_at: string | null;
  }[];
};

export type TagWithCount = { name: string; count: number };

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
