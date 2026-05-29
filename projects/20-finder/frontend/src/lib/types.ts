export type User = {
  id: string;
  email: string;
  name: string;
};

export type PlaceRow = {
  id: string;
  name: string;
  cuisine: string;
  address: string;
  lat: number;
  lng: number;
  distance_m: number | null;
  avg_rating: number | null;
  review_count: number;
  created_at: string;
  updated_at: string;
};

export type ReviewRow = {
  id: string;
  user_id: string;
  user_name: string;
  rating: number;
  body: string;
  created_at: string;
};

export type PlaceDetail = PlaceRow & {
  reviews: ReviewRow[];
};

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
