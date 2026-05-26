export type StreakInfo = {
  current: number;
  longest: number;
  total: number;
  last_completion: string | null;
};

export type Habit = {
  id: string;
  name: string;
  color: string;
  created_at: string;
  streak: StreakInfo;
  completions: string[];
};

export type ToggleResult = {
  completed: boolean;
  streak: StreakInfo;
  completions: string[];
};

export type ApiError = {
  error: {
    code: string;
    message: string;
  };
};

export const ALLOWED_COLORS = [
  '#4F46E5',
  '#059669',
  '#D97706',
  '#DC2626',
  '#0284C7',
  '#7C3AED',
  '#DB2777',
  '#0F766E'
] as const;
