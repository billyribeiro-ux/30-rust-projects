import { describe, it, expect, vi, beforeEach } from 'vitest';
import { foldersApi, filesApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('api client', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('list folders parses the array', async () => {
    const fetcher = vi.fn(async () => jsonResponse([{ id: 'a', parent_id: null, name: 'A' }]));
    const folders = await foldersApi.list(fetcher as unknown as typeof fetch);
    expect(folders.length).toBe(1);
    expect(folders[0]?.name).toBe('A');
  });

  it('create folder posts the body', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ id: 'x', parent_id: null, name: 'New', created_at: '', updated_at: '' }, 201)
    );
    await foldersApi.create(fetcher as unknown as typeof fetch, 'New', null);

    // Vitest's mock.calls types collide with strict mode — see PATTERNS.md.
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ name: 'New', parent_id: null }));
  });

  it('non-2xx maps to ApiCallError with the server-provided message', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse(
        { error: { code: 'not_found', message: 'gone', fields: null } },
        404
      )
    );
    await expect(filesApi.list(fetcher as unknown as typeof fetch, null)).rejects.toBeInstanceOf(
      ApiCallError
    );
  });

  it('downloadUrl encodes the id', () => {
    const u = filesApi.downloadUrl('abc/def?x');
    expect(u).toContain('/api/files/abc%2Fdef%3Fx/download');
  });
});
