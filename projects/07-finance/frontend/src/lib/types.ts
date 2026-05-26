export type AccountKind = 'asset' | 'liability' | 'income' | 'expense' | 'equity';

export type Account = {
  id: string;
  name: string;
  kind: AccountKind;
  color: string;
  created_at: string;
  balance_minor: number;
};

export type Posting = {
  id: string;
  account_id: string;
  amount_minor: number;
};

export type Transaction = {
  id: string;
  description: string;
  occurred_at: string;
  created_at: string;
  postings: Posting[];
};

export type ImportResponse = {
  imported: number;
  skipped: number;
  errors: string[];
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};

export const ALLOWED_COLORS = [
  '#4F46E5', '#059669', '#D97706', '#DC2626', '#0284C7', '#7C3AED', '#DB2777', '#0F766E'
] as const;

export const KIND_LABEL: Record<AccountKind, string> = {
  asset: 'Asset',
  liability: 'Liability',
  income: 'Income',
  expense: 'Expense',
  equity: 'Equity'
};
