import { describe, it, expect, vi, beforeEach } from 'vitest';
import { boardsApi, cardsApi, listsApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('kanban api client', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('boardsApi.list parses the array', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 'b1',
          owner_id: 'u1',
          name: 'Sprint 12',
          slug: 'sprint-12',
          role: 'admin',
          created_at: '2026-05-27T00:00:00Z',
          updated_at: '2026-05-27T00:00:00Z'
        }
      ])
    );
    const boards = await boardsApi.list(fetcher as unknown as typeof fetch);
    expect(boards.length).toBe(1);
    expect(boards[0]?.slug).toBe('sprint-12');
  });

  it('boardsApi.create posts the body', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse(
        {
          id: 'b2',
          owner_id: 'u1',
          name: 'New',
          slug: 'new',
          role: 'admin',
          created_at: '',
          updated_at: ''
        },
        201
      )
    );
    await boardsApi.create(fetcher as unknown as typeof fetch, 'New', 'new');
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ name: 'New', slug: 'new' }));
  });

  it('cardsApi.update encodes id and sends partial patch', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        id: 'c1',
        list_id: 'l2',
        title: 't',
        body: '',
        position: 2048,
        due_at: null,
        created_at: '',
        updated_at: ''
      })
    );
    await cardsApi.update(fetcher as unknown as typeof fetch, 'card 7', {
      list_id: 'l2',
      position: 2048
    });
    const [url, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(url).toContain('/api/cards/card%207');
    expect(init.method).toBe('PATCH');
  });

  it('listsApi.createCard posts to nested route', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        id: 'c2',
        list_id: 'l1',
        title: 'Hello',
        body: '',
        position: 1024,
        due_at: null,
        created_at: '',
        updated_at: ''
      })
    );
    await listsApi.createCard(fetcher as unknown as typeof fetch, 'l1', 'Hello', 1024);
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/lists/l1/cards');
  });

  it('non-2xx maps to ApiCallError with the server-provided message', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse(
        { error: { code: 'forbidden', message: 'viewer cannot edit', fields: null } },
        403
      )
    );
    await expect(
      cardsApi.update(fetcher as unknown as typeof fetch, 'c1', { title: 't' })
    ).rejects.toBeInstanceOf(ApiCallError);
  });
});
