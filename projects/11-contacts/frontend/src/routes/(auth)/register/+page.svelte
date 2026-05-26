<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();
  let submitting = $state(false);
</script>

<svelte:head><title>Create account — Contacts</title></svelte:head>

<h1>Create your account</h1>
<p class="lead">It takes about thirty seconds.</p>

<form
  method="POST"
  use:enhance={() => {
    submitting = true;
    return async ({ update }) => {
      submitting = false;
      await update();
    };
  }}
>
  <label class="field">
    <span>Name (optional)</span>
    <input type="text" name="name" autocomplete="name" maxlength="200" value={(form?.name as string | undefined) ?? ''} />
  </label>
  <label class="field">
    <span>Email</span>
    <input type="email" name="email" required autocomplete="email" value={(form?.email as string | undefined) ?? ''} />
  </label>
  <label class="field">
    <span>Password</span>
    <input type="password" name="password" required autocomplete="new-password" minlength="12" />
    <small>12 characters minimum.</small>
  </label>

  {#if form && 'error' in form && form.error}
    <p class="error" role="alert">{form.error}</p>
  {/if}

  <button type="submit" class="primary" disabled={submitting}>
    {submitting ? 'Creating…' : 'Create account'}
  </button>
</form>

<p class="alt">Already have an account? <a href="/login">Sign in</a></p>

<style>
  h1 { font-size: var(--text-2xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); margin-bottom: var(--space-5); }
  form { display: grid; gap: var(--space-4); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field small { color: var(--color-fg-muted); font-size: var(--text-xs); }
  .field input {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .primary {
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
  }
  .primary:hover:not(:disabled) { background: var(--color-accent-hover); }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
  .alt a { color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px; }
  .alt { margin-top: var(--space-5); color: var(--color-fg-muted); font-size: var(--text-sm); text-align: center; }
</style>
