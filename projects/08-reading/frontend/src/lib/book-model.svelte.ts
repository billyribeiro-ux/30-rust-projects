// A `.svelte.ts` file — Svelte 5 supports runes outside .svelte components
// when the filename ends with `.svelte.{ts,js}`. Letting us define a class
// whose instance fields ARE reactive `$state` slots, with computed
// `$derived` getters.
//
// This is the modern Svelte 5 "class with rune fields" pattern. It replaces
// the Svelte 4 "writable store + custom set/update + derived store" stack
// with a single, type-checked class declaration.

import type { Book } from './types';

export class BookModel {
  id: string;
  isbn: string | null;
  title = $state('');
  author = $state('');
  cover_url = $state<string | null>(null);
  pages = $state<number | null>(null);
  status = $state<Book['status']>('want_to_read');
  current_page = $state(0);
  started_at: string | null;
  finished_at: string | null;
  created_at: string;
  updated_at: string;

  /**
   * Percent progress. Derived from current_page + pages — changes
   * automatically whenever either is updated, without explicit subscriptions.
   */
  progress = $derived(this.pages && this.pages > 0
    ? Math.min(100, Math.round((this.current_page / this.pages) * 100))
    : 0);

  /** Sort order: reading first, then want_to_read, then finished. */
  statusOrder = $derived(
    this.status === 'reading' ? 0 : this.status === 'want_to_read' ? 1 : 2
  );

  constructor(book: Book) {
    this.id = book.id;
    this.isbn = book.isbn;
    this.title = book.title;
    this.author = book.author;
    this.cover_url = book.cover_url;
    this.pages = book.pages;
    this.status = book.status;
    this.current_page = book.current_page;
    this.started_at = book.started_at;
    this.finished_at = book.finished_at;
    this.created_at = book.created_at;
    this.updated_at = book.updated_at;
  }

  /** Snapshot the reactive fields back to a plain JSON shape, e.g., for
   * sending in an UpdateBook payload. */
  toUpdatePayload(): { status: Book['status']; current_page: number; title: string; author: string } {
    return {
      status: this.status,
      current_page: this.current_page,
      title: this.title,
      author: this.author
    };
  }
}
