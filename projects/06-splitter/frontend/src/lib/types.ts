export type Member = {
  id: string;
  name: string;
  color: string;
  created_at: string;
};

export type SplitKind = 'equal' | 'exact' | 'percent';

export type ShareInput = {
  member_id: string;
  value: number; // cents for exact, basis points (0..10000) for percent, ignored for equal
};

export type ShareOutput = {
  member_id: string;
  share_cents: number;
};

export type Expense = {
  id: string;
  payer_id: string;
  amount_cents: number;
  description: string;
  paid_at: string;
  split_kind: SplitKind;
  created_at: string;
  shares: ShareOutput[];
};

export type Balance = {
  member_id: string;
  name: string;
  paid_cents: number;
  owed_cents: number;
  net_cents: number;
};

export type Settlement = {
  from: string;
  to: string;
  cents: number;
};

export type BalancesResponse = {
  balances: Balance[];
  settlements: Settlement[];
};

export type FieldError = {
  field: string;
  message: string;
};

export type ApiError = {
  error: {
    code: string;
    message: string;
    fields: FieldError[] | null;
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
