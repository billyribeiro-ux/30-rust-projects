export type User = { id: string; email: string; name: string };

export type TopKind = { kind: string; count: number };
export type KpiSnapshot = {
  events_last_minute: number;
  events_last_hour: number;
  unique_sessions_last_hour: number;
  top_kinds: TopKind[];
  ts: string;
};

export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
