export type User = { id: string; email: string; name: string };
export type CourseSummary = {
  id: string;
  slug: string;
  title: string;
  summary: string;
  price_cents: number;
  currency: string;
  status: string;
  instructor_name: string;
  created_at: string;
};
export type Lesson = {
  id: string;
  title: string;
  position: number;
  duration_seconds: number | null;
};
export type CourseDetail = {
  course: CourseSummary;
  lessons: Lesson[];
  enrolled: boolean;
};
export type Instructor = {
  user_id: string;
  stripe_account_id: string | null;
  payouts_enabled: boolean;
  details_submitted: boolean;
  created_at: string;
};
export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
