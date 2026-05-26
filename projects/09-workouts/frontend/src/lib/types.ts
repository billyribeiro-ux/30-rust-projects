export type MuscleGroup =
  | 'chest'
  | 'back'
  | 'legs'
  | 'shoulders'
  | 'arms'
  | 'core'
  | 'other';

export const MUSCLE_GROUPS: MuscleGroup[] = [
  'chest',
  'back',
  'legs',
  'shoulders',
  'arms',
  'core',
  'other'
];

export type Exercise = {
  id: string;
  name: string;
  muscle_group: MuscleGroup;
  created_at: string;
};

export type WorkoutSet = {
  id: string;
  exercise_id: string;
  exercise_name: string;
  weight_minor: number;
  reps: number;
  rir: number;
  set_order: number;
  volume_minor: number;
  is_pr: boolean;
};

export type Workout = {
  id: string;
  name: string;
  performed_at: string;
  created_at: string;
  sets: WorkoutSet[];
};

export type WorkoutSummary = {
  id: string;
  name: string;
  performed_at: string;
  created_at: string;
  set_count: number;
  total_volume_minor: number;
};

export type Stats = {
  total_sets: number;
  total_volume_minor: number;
  workouts_last_7_days: number;
  pr_count_total: number;
};

export type PrRow = {
  set_id: string;
  workout_id: string;
  weight_minor: number;
  reps: number;
  rir: number;
  volume_minor: number;
  performed_at: string;
};

export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
