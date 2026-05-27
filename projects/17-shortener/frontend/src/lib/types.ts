export type User = {
  id: string;
  email: string;
  name: string;
  email_verified: boolean;
};

export type Link = {
  id: string;
  slug: string;
  target_url: string;
  created_at: string;
  click_count: number | null;
};

export type NamedCount = { name: string; count: number };
export type DailyBucket = { day: string; count: number };

export type Stats = {
  link: Link;
  total_clicks: number;
  clicks_today: number;
  clicks_7d: number;
  clicks_30d: number;
  redis_counter: number | null;
  top_referers: NamedCount[];
  top_countries: NamedCount[];
  daily: DailyBucket[];
};

export type TwoFASetup = {
  provisioning_uri: string;
  qr_png_base64: string;
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
