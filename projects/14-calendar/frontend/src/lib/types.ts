export type User = {
  id: string;
  email: string;
  name: string;
  email_verified: boolean;
};

export type Permission = 'owner' | 'edit' | 'view';

export type Calendar = {
  id: string;
  owner_id: string;
  name: string;
  color: string;
  default_tz: string;
  permission: Permission;
  created_at: string;
  updated_at: string;
};

export type Occurrence = {
  event_id: string;
  calendar_id: string;
  title: string;
  description: string;
  location: string;
  start_at: string;
  end_at: string;
  tz: string;
  all_day: boolean;
  is_recurring: boolean;
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
