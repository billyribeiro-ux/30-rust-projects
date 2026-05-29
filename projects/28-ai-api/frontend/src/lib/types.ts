export type User = { id: string; email: string; name: string };
export type ApiKey = {
  id: string;
  name: string;
  prefix: string;
  scopes: string[];
  rpm_limit: number;
  revoked: boolean;
  created_at: string;
  last_used_at: string | null;
};
export type Usage = {
  calls_this_month: number;
  estimated_cents: number;
  free_tier_calls: number;
  per_call_cents: number;
  last_call_at: string | null;
};
export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
