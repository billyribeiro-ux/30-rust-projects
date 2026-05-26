export type SessionKind = 'work' | 'short_break' | 'long_break';

export type Session = {
  id: string;
  kind: SessionKind;
  label: string | null;
  planned_seconds: number;
  actual_seconds: number;
  started_at: string;
  ended_at: string;
};

export type Stats = {
  focus_seconds_today: number;
  focus_seconds_week: number;
  sessions_today: number;
  pomodoros_today: number;
};

export type ApiError = {
  error: { code: string; message: string };
};

export const KIND_LABEL: Record<SessionKind, string> = {
  work: 'Focus',
  short_break: 'Short break',
  long_break: 'Long break'
};

export const KIND_SECONDS: Record<SessionKind, number> = {
  work: 25 * 60,
  short_break: 5 * 60,
  long_break: 15 * 60
};

export const KIND_COLOR: Record<SessionKind, string> = {
  work: '#DC2626',
  short_break: '#059669',
  long_break: '#0284C7'
};
