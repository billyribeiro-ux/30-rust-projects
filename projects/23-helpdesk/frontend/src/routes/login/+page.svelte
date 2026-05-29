<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();
  let submitting = $state(false);
</script>

<svelte:head><title>Sign in — Help Desk</title></svelte:head>

<main class="shell">
  <section class="card">
    <h1>Sign in</h1>
    <p class="lead">Agents and admins use a password. Customers can <a href="/magic">use a magic link</a>.</p>
    <form
      method="POST"
      use:enhance={() => {
        submitting = true;
        return async ({ update }) => { submitting = false; await update(); };
      }}
    >
      <label class="field">
        <span>Email</span>
        <input type="email" name="email" required autocomplete="email" value={(form?.email as string | undefined) ?? ''} />
      </label>
      <label class="field">
        <span>Password</span>
        <input type="password" name="password" required autocomplete="current-password" minlength="12" />
      </label>
      {#if form && 'error' in form && form.error}
        <p class="error" role="alert">{form.error as string}</p>
      {/if}
      <button type="submit" class="primary" disabled={submitting}>{submitting ? 'Signing in…' : 'Sign in'}</button>
    </form>
    <p class="alt">No account? <a href="/signup">Create one</a></p>
  </section>
</main>

<style>
  .shell { min-height: 100dvh; display: grid; place-items: center; padding: var(--space-4); background: var(--color-bg-sunken); }
  .card { width: 100%; max-width: 420px; padding: var(--space-8) var(--space-6); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-lg); box-shadow: var(--shadow-md); display: grid; gap: var(--space-3); }
  h1 { font-size: var(--text-2xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .lead a { color: var(--color-accent); }
  form { display: grid; gap: var(--space-3); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field input { padding: var(--space-3) var(--space-4); background: var(--color-bg); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
  .primary { padding: var(--space-3) var(--space-4); background: var(--color-accent); color: var(--color-accent-fg); border-radius: var(--radius-md); font-weight: 600; }
  .primary:disabled { opacity: 0.5; }
  .error { padding: var(--space-3) var(--space-4); color: var(--color-danger); background: hsl(0 72% 51% / 0.08); border: 1px solid hsl(0 72% 51% / 0.3); border-radius: var(--radius-md); font-size: var(--text-sm); }
  .alt { text-align: center; color: var(--color-fg-muted); font-size: var(--text-sm); }
  .alt a { color: var(--color-accent); }
</style>
