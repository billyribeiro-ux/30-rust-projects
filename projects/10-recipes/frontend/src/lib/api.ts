import type {
  Recipe,
  RecipeSummary,
  RecipeImage,
  Rating,
  ShareLink,
  ApiError,
  FieldError
} from './types';

export const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3009';

type FetchLike = typeof fetch;

export class ApiCallError extends Error {
  status: number;
  fields: FieldError[] | null;
  constructor(message: string, status: number, fields: FieldError[] | null) {
    super(message);
    this.status = status;
    this.fields = fields;
  }
}

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

export type CreateRecipeInput = {
  title: string;
  description?: string;
  ingredients?: string[];
  instructions?: string[];
  prep_minutes?: number | null;
  cook_minutes?: number | null;
  servings?: number | null;
};

export type UpdateRecipeInput = Partial<CreateRecipeInput & { cover_image_id: string }>;

export const recipesApi = {
  list: (f: FetchLike) => request<RecipeSummary[]>(f, '/api/recipes'),
  getBySlug: (f: FetchLike, slug: string) =>
    request<Recipe>(f, `/api/recipes/by-slug/${encodeURIComponent(slug)}`),
  create: (f: FetchLike, input: CreateRecipeInput) =>
    request<Recipe>(f, '/api/recipes', { method: 'POST', body: JSON.stringify(input) }),
  update: (f: FetchLike, id: string, input: UpdateRecipeInput) =>
    request<Recipe>(f, `/api/recipes/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(input)
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/recipes/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  share: (f: FetchLike, id: string) =>
    request<ShareLink>(f, `/api/recipes/${encodeURIComponent(id)}/share`, { method: 'POST' }),
  getShared: (f: FetchLike, slug: string, sig: string, exp: number) =>
    request<Recipe>(
      f,
      `/api/share/${encodeURIComponent(slug)}?sig=${encodeURIComponent(sig)}&exp=${exp}`
    )
};

export const imagesApi = {
  // Multipart upload — note the absence of the JSON `content-type` header.
  // Letting `fetch` set the boundary itself (or letting the SvelteKit remote
  // function's form() pipe straight through) is mandatory; if we set it
  // ourselves we miss the `; boundary=...` part and the server can't parse.
  upload: async (f: FetchLike, recipeId: string, file: File): Promise<RecipeImage> => {
    const fd = new FormData();
    fd.append('file', file);
    const res = await f(`${API_BASE}/api/recipes/${encodeURIComponent(recipeId)}/images`, {
      method: 'POST',
      body: fd
    });
    const body = await res.json().catch(() => null);
    if (!res.ok) {
      const err = body as ApiError | null;
      throw new ApiCallError(
        err?.error?.message ?? `upload failed: ${res.status}`,
        res.status,
        err?.error?.fields ?? null
      );
    }
    return body as RecipeImage;
  },
  remove: (f: FetchLike, imageId: string) =>
    request<void>(f, `/api/images/${encodeURIComponent(imageId)}`, { method: 'DELETE' })
};

export const ratingsApi = {
  list: (f: FetchLike, recipeId: string) =>
    request<Rating[]>(f, `/api/recipes/${encodeURIComponent(recipeId)}/ratings`),
  create: (f: FetchLike, recipeId: string, input: { stars: number; comment?: string }) =>
    request<Rating>(f, `/api/recipes/${encodeURIComponent(recipeId)}/ratings`, {
      method: 'POST',
      body: JSON.stringify(input)
    })
};
