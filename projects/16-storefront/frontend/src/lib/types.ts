export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};

export type Product = {
  id: string;
  sku: string;
  name: string;
  description: string;
  price_cents: number;
  currency: string;
};

export type CheckoutResponse = {
  url: string;
  session_id: string;
  order_id: string;
};

export type Order = {
  id: string;
  customer_email: string;
  stripe_session_id: string;
  stripe_payment_intent: string | null;
  status: 'pending' | 'paid' | 'fulfilled' | 'refunded' | 'failed';
  amount_cents: number;
  currency: string;
  refunded_at: string | null;
  fulfilled_at: string | null;
  created_at: string;
};

export type AdminProduct = Product & {
  file_path: string | null;
  file_name: string | null;
  file_size: number | null;
  active: boolean;
  created_at: string;
  updated_at: string;
};

export type AdminUser = {
  id: string;
  email: string;
  name: string;
  email_verified: boolean;
};
