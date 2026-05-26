export type RecipeImage = {
  id: string;
  recipe_id: string;
  mime_type: string;
  width: number;
  height: number;
  bytes: number;
  url: string;
  thumb_url: string;
};

export type RecipeSummary = {
  id: string;
  slug: string;
  title: string;
  description: string;
  prep_minutes: number | null;
  cook_minutes: number | null;
  servings: number | null;
  cover_thumb_url: string | null;
  created_at: string;
  updated_at: string;
};

export type Recipe = {
  id: string;
  slug: string;
  title: string;
  description: string;
  ingredients: string[];
  instructions: string[];
  prep_minutes: number | null;
  cook_minutes: number | null;
  servings: number | null;
  cover_image_id: string | null;
  images: RecipeImage[];
  created_at: string;
  updated_at: string;
};

export type Rating = {
  id: string;
  recipe_id: string;
  stars: number;
  comment: string;
  created_at: string;
};

export type ShareLink = {
  url: string;
  expires_at: string;
};

export type FieldError = { field: string; message: string };
export type ApiError = { error: { code: string; message: string; fields: FieldError[] | null } };
