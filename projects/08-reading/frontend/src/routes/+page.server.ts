import { fail } from '@sveltejs/kit';
import {
  booksApi,
  lookupApi,
  ApiCallError,
  type CreateBookInput
} from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const books = await booksApi.list(fetch);

  // STREAMED RETURN PATTERN: aggregate stats are computed from the book list,
  // but to demonstrate streamed loads we return a promise for an "extra"
  // payload that the page can {#await} separately. SvelteKit will flush the
  // synchronous parts of the response first and stream the promise body
  // later, so the page paints with the book grid before the secondary
  // section resolves.
  const stats = computeStats(books);

  return {
    books,
    // Wrap in a promise we deliberately yield to demonstrate the streamed
    // pattern. In a real app this would be a network call that's slower
    // than the primary one (e.g., aggregate analytics).
    streamed: {
      stats: new Promise<typeof stats>((resolve) => {
        setTimeout(() => resolve(stats), 50);
      })
    }
  };
};

function computeStats(books: { status: string; current_page: number; pages: number | null }[]) {
  const total = books.length;
  const reading = books.filter((b) => b.status === 'reading').length;
  const finished = books.filter((b) => b.status === 'finished').length;
  const pagesRead = books.reduce((acc, b) => acc + (b.current_page ?? 0), 0);
  return { total, reading, finished, pagesRead };
}

export const actions: Actions = {
  addBook: async ({ request, fetch }) => {
    const data = await request.formData();
    const isbn = String(data.get('isbn') ?? '').trim() || null;
    const title = String(data.get('title') ?? '').trim();
    const author = String(data.get('author') ?? '').trim();
    const status = String(data.get('status') ?? 'want_to_read') as CreateBookInput['status'];
    if (!title) return fail(422, { kind: 'addBook', error: 'Title is required.', isbn, title, author });
    try {
      await booksApi.create(fetch, { isbn, title, author, status });
      return { success: true, kind: 'addBook' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'addBook', error: err.message, isbn, title, author });
      }
      throw err;
    }
  },

  lookupIsbn: async ({ request, fetch }) => {
    const isbn = String((await request.formData()).get('isbn') ?? '').trim();
    if (!isbn) return fail(422, { kind: 'lookupIsbn', error: 'Enter an ISBN.' });
    try {
      const result = await lookupApi.isbn(fetch, isbn);
      // Auto-create a book from the lookup.
      await booksApi.create(fetch, {
        isbn: result.isbn,
        title: result.title,
        author: result.author,
        cover_url: result.cover_url,
        pages: result.pages,
        status: 'want_to_read'
      });
      return { success: true, kind: 'lookupIsbn', lookup: result };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'lookupIsbn', error: err.message, isbn });
      }
      throw err;
    }
  },

  removeBook: async ({ request, fetch }) => {
    const id = String((await request.formData()).get('id') ?? '');
    if (!id) return fail(422, { kind: 'removeBook', error: 'Missing id.' });
    try {
      await booksApi.remove(fetch, id);
      return { success: true, kind: 'removeBook' };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { kind: 'removeBook', error: err.message });
      }
      throw err;
    }
  }
};
