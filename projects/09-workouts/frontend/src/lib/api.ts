import type {
  Exercise,
  Workout,
  WorkoutSummary,
  Stats,
  PrRow,
  ApiError,
  MuscleGroup
} from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3008';
type FetchLike = typeof fetch;

async function request<T>(fetcher: FetchLike, path: string, init?: RequestInit): Promise<T> {
  const res = await fetcher(`${API_BASE}${path}`, {
    ...init,
    headers: {
      'content-type': 'application/json',
      accept: 'application/json',
      ...(init?.headers ?? {})
    }
  });
  if (res.status === 204) return undefined as T;
  const body = await res.json().catch(() => null);
  if (!res.ok) {
    const err = body as ApiError | null;
    throw new ApiCallError(
      err?.error?.message ?? `request failed: ${res.status}`,
      res.status,
      err?.error?.fields ?? null
    );
  }
  return body as T;
}

export class ApiCallError extends Error {
  status: number;
  fields: { field: string; message: string }[] | null;
  constructor(
    message: string,
    status: number,
    fields: { field: string; message: string }[] | null
  ) {
    super(message);
    this.status = status;
    this.fields = fields;
  }
}

export type CreateExerciseInput = { name: string; muscle_group: MuscleGroup };

export type CreateSetInput = {
  exercise_id: string;
  weight_minor: number;
  reps: number;
  rir?: number;
};

export type CreateWorkoutInput = {
  name?: string;
  performed_at?: string;
  sets: CreateSetInput[];
};

export const exercisesApi = {
  list: (f: FetchLike, q?: string) =>
    request<Exercise[]>(
      f,
      q && q.trim() ? `/api/exercises?q=${encodeURIComponent(q.trim())}` : '/api/exercises'
    ),
  create: (f: FetchLike, input: CreateExerciseInput) =>
    request<Exercise>(f, '/api/exercises', { method: 'POST', body: JSON.stringify(input) }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/exercises/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  prs: (f: FetchLike, id: string) =>
    request<PrRow[]>(f, `/api/exercises/${encodeURIComponent(id)}/prs`)
};

export const workoutsApi = {
  list: (f: FetchLike) => request<WorkoutSummary[]>(f, '/api/workouts'),
  get: (f: FetchLike, id: string) =>
    request<Workout>(f, `/api/workouts/${encodeURIComponent(id)}`),
  create: (f: FetchLike, input: CreateWorkoutInput) =>
    request<Workout>(f, '/api/workouts', { method: 'POST', body: JSON.stringify(input) }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/workouts/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const statsApi = {
  read: (f: FetchLike) => request<Stats>(f, '/api/stats')
};
