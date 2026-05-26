/**
 * Remote functions for photo management.
 *
 * `$app/server` is the new SvelteKit remote-functions API. It lets you
 * write server-only code that the browser calls type-safely, without
 * the FormData/JSON shuffle of `+page.server.ts` actions or the boilerplate
 * of `+server.ts` endpoint handlers.
 *
 * Three flavors:
 * - `query(...)`   — a server-side READ. Returns a typed value. Can be
 *                    refreshed from the client; `await query()` from
 *                    a Svelte component subscribes the page to it.
 * - `form(...)`    — a server-side WRITE bound to a `<form>` element.
 *                    Spread onto a `<form>`: progressive-enhancement
 *                    submit + typed validation issues for free.
 * - `command(...)` — a server-side WRITE called imperatively from
 *                    client TypeScript. Like a typed RPC.
 *
 * We use all three to demonstrate the pattern:
 *  - `listPhotos`   (query)   — fetch images for one recipe.
 *  - `uploadPhoto`  (form)    — multipart upload via <form>.
 *  - `removePhoto`  (command) — imperative delete on a button click.
 *
 * Validators: SvelteKit accepts any Standard-Schema validator
 * (valibot, zod, arktype, …). For this lesson we declare the arg
 * schema as `'unchecked'` and validate inside the body — keeps deps
 * small and the code reads more honestly. A real app would wire in
 * zod or valibot here.
 *
 * Note: the multipart upload happens on the SvelteKit server, which
 * then forwards the FormData to the Rust backend. This is the "BFF"
 * (backend-for-frontend) pattern — the browser never talks directly
 * to Rust, which lets us inject auth headers / hide the backend URL
 * later without changing the UI code.
 */

import { query, form, command, getRequestEvent } from '$app/server';
import { recipesApi, imagesApi, ApiCallError } from '$lib/api';
import type { RecipeImage } from '$lib/types';

type UploadResult =
  | { ok: true; image: RecipeImage }
  | { ok: false; error: string };

type RemoveResult = { ok: true } | { ok: false; error: string };

// ---- query: listPhotos ----

export const listPhotos = query(
  'unchecked',
  async (recipeId: string): Promise<RecipeImage[]> => {
    const { fetch } = getRequestEvent();
    // Look up the recipe (it's small, paginated, so list+find is fine).
    const all = await recipesApi.list(fetch);
    const summary = all.find((r) => r.id === recipeId);
    if (!summary) return [];
    const full = await recipesApi.getBySlug(fetch, summary.slug);
    return full.images;
  }
);

// ---- form: uploadPhoto ----

export const uploadPhoto = form(
  'unchecked',
  async (data: Record<string, unknown>): Promise<UploadResult> => {
    const { fetch } = getRequestEvent();
    const recipe_id = typeof data.recipe_id === 'string' ? data.recipe_id : '';
    const file = data.file;
    if (!recipe_id) {
      return { ok: false, error: 'Missing recipe id.' };
    }
    if (!(file instanceof File) || file.size === 0) {
      return { ok: false, error: 'No file selected.' };
    }
    try {
      const image = await imagesApi.upload(fetch, recipe_id, file);
      // Tell the live query its cached result is stale.
      await listPhotos(recipe_id).refresh();
      return { ok: true, image };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return { ok: false, error: err.message };
      }
      throw err;
    }
  }
);

// ---- command: removePhoto ----

export const removePhoto = command(
  'unchecked',
  async (arg: { image_id: string; recipe_id: string }): Promise<RemoveResult> => {
    const { fetch } = getRequestEvent();
    try {
      await imagesApi.remove(fetch, arg.image_id);
      await listPhotos(arg.recipe_id).refresh();
      return { ok: true };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return { ok: false, error: err.message };
      }
      throw err;
    }
  }
);
