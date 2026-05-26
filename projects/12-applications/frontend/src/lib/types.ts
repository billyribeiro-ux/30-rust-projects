export type User = {
  id: string;
  email: string;
  name: string;
  email_verified: boolean;
};

export type AppStatus =
  | 'wishlist'
  | 'applied'
  | 'screening'
  | 'interview'
  | 'offer'
  | 'accepted'
  | 'rejected'
  | 'withdrawn';

export const STATUS_LABEL: Record<AppStatus, string> = {
  wishlist: 'Wishlist',
  applied: 'Applied',
  screening: 'Screening',
  interview: 'Interview',
  offer: 'Offer',
  accepted: 'Accepted',
  rejected: 'Rejected',
  withdrawn: 'Withdrawn'
};

export type Application = {
  id: string;
  company: string;
  role: string;
  location: string;
  salary_min: number | null;
  salary_max: number | null;
  job_url: string;
  notes: string;
  status: AppStatus;
  applied_at: string | null;
  created_at: string;
  updated_at: string;
};

export type ApplicationEvent = {
  id: string;
  kind: 'status_change' | 'note' | 'contact_made' | 'rejected' | 'offer_received' | 'withdrew';
  body: string;
  new_status: AppStatus | null;
  occurred_at: string;
};

export type NextStep = {
  id: string;
  application_id: string;
  body: string;
  due_at: string;
  completed_at: string | null;
  reminded_at: string | null;
  created_at: string;
};

export type DashboardData = {
  total_applications: number;
  by_status: { status: AppStatus; count: number }[];
  active_pipeline: number;
  due_this_week: {
    id: string;
    application_id: string;
    company: string;
    body: string;
    due_at: string;
  }[];
  recent_events: {
    application_id: string;
    company: string;
    role: string;
    kind: string;
    body: string;
    new_status: AppStatus | null;
    occurred_at: string;
  }[];
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
