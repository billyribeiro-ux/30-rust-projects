import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tenantsApi, projectsApi, tasksApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('capstone api', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('tenantsApi.list parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 't',
          slug: 'acme',
          name: 'Acme',
          role: 'owner',
          plan: 'free',
          status: 'active',
          created_at: ''
        }
      ])
    );
    const out = await tenantsApi.list(fetcher as unknown as typeof fetch);
    expect(out[0]?.role).toBe('owner');
  });

  it('projectsApi.create posts body and encodes slug', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ id: 'x' }, 201));
    await projectsApi.create(fetcher as unknown as typeof fetch, 'a b', 'web', 'Web');
    const [url, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(url).toContain('/api/t/a%20b/projects');
    expect(init.body).toBe(JSON.stringify({ slug: 'web', name: 'Web' }));
  });

  it('tasksApi.list builds the right URL', async () => {
    const fetcher = vi.fn(async () => jsonResponse([]));
    await tasksApi.list(fetcher as unknown as typeof fetch, 'acme', 'pid');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/t/acme/projects/pid/tasks');
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'forbidden', message: 'x', fields: null } }, 403)
    );
    await expect(tenantsApi.list(fetcher as unknown as typeof fetch)).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
