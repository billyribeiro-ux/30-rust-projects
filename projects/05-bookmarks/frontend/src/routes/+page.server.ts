import { fail } from '@sveltejs/kit';
import { bookmarksApi, tagsApi, ApiCallError, type CreateBookmarkInput } from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const q = url.searchParams.get('q') ?? '';
  const tag = url.searchParams.get('tag') ?? '';
  const [bookmarks, tags] = await Promise.all([
    bookmarksApi.list(fetch, { q: q || undefined, tag: tag || undefined }),
    tagsApi.list(fetch)
  ]);
  return { bookmarks, tags, q, tag };
};

export const actions: Actions = {
  create: async ({ request, fetch }) => {
    const data = await request.formData();
    const tagsRaw = String(data.get('tags') ?? '');
    const tags = tagsRaw
      .split(/[,\s]+/)
      .map((s) => s.trim())
      .filter(Boolean);

    const input: CreateBookmarkInput = {
      url: String(data.get('url') ?? '').trim(),
      title: String(data.get('title') ?? '').trim(),
      description: String(data.get('description') ?? '').trim(),
      tags
    };

    if (!input.url) return fail(422, { error: 'URL must not be empty.', ...input, tagsRaw });
    if (!input.title) return fail(422, { error: 'Title must not be empty.', ...input, tagsRaw });

    try {
      await bookmarksApi.create(fetch, input);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message, ...input, tagsRaw });
      throw err;
    }
  },

  remove: async ({ request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { error: 'Missing id.' });
    try {
      await bookmarksApi.remove(fetch, id);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};
