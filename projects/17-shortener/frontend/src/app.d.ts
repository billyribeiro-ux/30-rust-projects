// See https://svelte.dev/docs/kit/types#app
declare global {
  namespace App {
    interface Locals {
      /** Populated by hooks.server.ts via /api/auth/me. `null` for anonymous. */
      user: import('$lib/types').User | null;
      /** Raw session cookie value, kept on locals so server-side fetches
       * can forward it to the Rust backend without re-reading event.cookies. */
      sessionCookie: string | null;
    }
  }
}

export {};
