import { marked } from 'marked';

marked.setOptions({
  gfm: true,
  breaks: false
});

/**
 * Client-side preview render. The author is previewing their own input,
 * so XSS is not a concern here (they would only attack themselves).
 * Persistence + display of others' notes is rendered by the backend,
 * which sanitizes with ammonia. Never feed `preview()` output into the DOM
 * of a page that shows OTHER users' notes.
 */
export function preview(md: string): string {
  return marked.parse(md, { async: false }) as string;
}
