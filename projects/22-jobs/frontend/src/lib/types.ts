export type User = { id: string; email: string; name: string };

export type QueueSummary = {
  queue: string;
  pending: number;
  running: number;
  succeeded: number;
  failed: number;
  dead: number;
};

export type JobOut = {
  id: string;
  queue: string;
  kind: string;
  attempts: number;
  max_attempts: number;
  status: string;
  run_at: string;
  last_error: string | null;
  created_at: string;
  updated_at: string;
};

export type JobEvent = {
  job_id: string;
  status: string;
  queue: string;
  kind: string;
  attempts: number;
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
