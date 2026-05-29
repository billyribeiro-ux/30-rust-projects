declare global {
  namespace App {
    interface Locals {
      user: import('$lib/types').User | null;
      sessionCookie: string | null;
    }
  }
}

export {};
