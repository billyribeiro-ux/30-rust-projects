export type User = { id: string; email: string; name: string };
export type Interview = {
  id: string;
  interviewer_id: string;
  candidate_email: string;
  status: 'scheduled' | 'live' | 'ended';
  code: string;
  language: string;
  started_at: string | null;
  ended_at: string | null;
  created_at: string;
};
export type ExecutionOut = {
  id: string;
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
};
export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
