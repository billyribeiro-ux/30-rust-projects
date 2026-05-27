/**
 * Chunked uploader (tus-style, simplified) with progress events,
 * exponential-backoff retry, and localStorage-backed resumability.
 *
 * The protocol matches our backend:
 *   1. POST   /api/uploads        { filename, size, folder_id, content_type }
 *      → { upload_id }
 *   2. HEAD   /api/uploads/{id}   → returns Upload-Offset
 *   3. PATCH  /api/uploads/{id}   body = next chunk, Upload-Offset: N
 *      → 204 + Upload-Offset (intermediate)  OR  201 + X-File-Id (final)
 *
 * Resumability: keyed by `vault.upload.${filename}.${size}`. If the same
 * (file, size) pair was being uploaded last time and the page was closed,
 * we look up the saved upload_id, HEAD it, and resume from the server's
 * recorded offset. This survives full browser restarts.
 */

import { backendUrl } from './api';

export type UploadProgress = {
  loaded: number;
  total: number;
  percent: number;
};

export type UploadEvents = {
  onProgress?: (p: UploadProgress) => void;
  onChunk?: (offset: number) => void;
  signal?: AbortSignal;
};

const CHUNK_SIZE = 1 * 1024 * 1024; // 1 MiB
const MAX_RETRIES = 3;
const BASE_BACKOFF_MS = 250;

function lsKey(filename: string, size: number) {
  return `vault.upload.${filename}.${size}`;
}

/** Saved alongside the in-progress upload so we can resume after reload. */
type SavedUpload = { upload_id: string; folder_id: string | null };

function savedUpload(filename: string, size: number): SavedUpload | null {
  try {
    const raw = globalThis.localStorage?.getItem(lsKey(filename, size));
    return raw ? (JSON.parse(raw) as SavedUpload) : null;
  } catch {
    return null;
  }
}

function saveUpload(filename: string, size: number, value: SavedUpload) {
  try {
    globalThis.localStorage?.setItem(lsKey(filename, size), JSON.stringify(value));
  } catch {
    /* ignore quota / private mode errors */
  }
}

function clearSavedUpload(filename: string, size: number) {
  try {
    globalThis.localStorage?.removeItem(lsKey(filename, size));
  } catch {
    /* ignore */
  }
}

async function createSession(
  filename: string,
  size: number,
  folder_id: string | null,
  content_type: string,
  signal?: AbortSignal
): Promise<string> {
  const res = await fetch(`${backendUrl()}/api/uploads`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ filename, size, folder_id, content_type }),
    signal
  });
  if (!res.ok) throw new Error(`create upload: ${res.status}`);
  const body = (await res.json()) as { upload_id: string };
  return body.upload_id;
}

async function headOffset(upload_id: string, signal?: AbortSignal): Promise<number | null> {
  const res = await fetch(`${backendUrl()}/api/uploads/${upload_id}`, {
    method: 'HEAD',
    credentials: 'include',
    signal
  });
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`head: ${res.status}`);
  const offset = Number(res.headers.get('upload-offset') ?? '0');
  return Number.isFinite(offset) ? offset : null;
}

/**
 * PATCH one chunk with exponential-backoff retry. Returns the new offset
 * the server reports back. On the final chunk the server returns 201
 * with an `x-file-id` header; the caller resolves that elsewhere by
 * comparing the returned offset with `total`.
 */
async function patchChunk(
  upload_id: string,
  offset: number,
  chunk: Blob,
  signal?: AbortSignal
): Promise<{ newOffset: number; fileId: string | null }> {
  let attempt = 0;
  let lastErr: unknown = null;
  while (attempt <= MAX_RETRIES) {
    if (signal?.aborted) throw new Error('aborted');
    try {
      const res = await fetch(`${backendUrl()}/api/uploads/${upload_id}`, {
        method: 'PATCH',
        credentials: 'include',
        headers: {
          'content-type': 'application/offset+octet-stream',
          'upload-offset': String(offset)
        },
        body: chunk,
        signal
      });
      if (res.status === 204 || res.status === 201) {
        const newOffset = Number(res.headers.get('upload-offset') ?? String(offset + chunk.size));
        const fileId = res.headers.get('x-file-id');
        return { newOffset, fileId };
      }
      if (res.status === 409) {
        // server says our offset is stale; let the caller HEAD again.
        throw new ConflictError('offset mismatch');
      }
      throw new Error(`patch: ${res.status}`);
    } catch (err) {
      if (err instanceof ConflictError) throw err;
      if (signal?.aborted) throw err;
      lastErr = err;
      attempt += 1;
      if (attempt > MAX_RETRIES) break;
      const wait = BASE_BACKOFF_MS * 2 ** (attempt - 1);
      await new Promise((r) => setTimeout(r, wait));
    }
  }
  throw lastErr ?? new Error('patch chunk failed');
}

class ConflictError extends Error {}

export type UploadResult = { file_id: string | null };

/** Drive a chunked upload to completion. */
export async function uploadFile(
  file: File,
  folder_id: string | null,
  events: UploadEvents = {}
): Promise<UploadResult> {
  const filename = file.name;
  const size = file.size;
  const content_type = file.type || 'application/octet-stream';
  const { onProgress, onChunk, signal } = events;

  // Resumable session lookup
  let upload_id: string | null = null;
  const saved = savedUpload(filename, size);
  if (saved) {
    const off = await headOffset(saved.upload_id, signal);
    if (off !== null) {
      upload_id = saved.upload_id;
    } else {
      clearSavedUpload(filename, size);
    }
  }
  if (!upload_id) {
    upload_id = await createSession(filename, size, folder_id, content_type, signal);
    saveUpload(filename, size, { upload_id, folder_id });
  }

  let offset = (await headOffset(upload_id, signal)) ?? 0;
  onProgress?.({ loaded: offset, total: size, percent: size ? offset / size : 1 });

  let fileId: string | null = null;
  while (offset < size) {
    if (signal?.aborted) throw new Error('aborted');
    const end = Math.min(offset + CHUNK_SIZE, size);
    const chunk = file.slice(offset, end);
    try {
      const r = await patchChunk(upload_id, offset, chunk, signal);
      offset = r.newOffset;
      fileId = r.fileId ?? fileId;
      onChunk?.(offset);
      onProgress?.({ loaded: offset, total: size, percent: size ? offset / size : 1 });
    } catch (err) {
      if (err instanceof ConflictError) {
        const fresh = await headOffset(upload_id, signal);
        if (fresh === null) {
          clearSavedUpload(filename, size);
          throw new Error('upload session lost');
        }
        offset = fresh;
        continue;
      }
      throw err;
    }
  }

  clearSavedUpload(filename, size);
  return { file_id: fileId };
}
