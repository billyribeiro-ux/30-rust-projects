<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();
</script>

<svelte:head><title>Reset password — Applications</title></svelte:head>

<h1>Reset your password</h1>
<p class="lead">Enter your email and we'll send you a reset link.</p>

<form method="POST" use:enhance>
  <label class="field">
    <span>Email</span>
    <input type="email" name="email" required autocomplete="email" />
  </label>
  <button type="submit" class="primary">Send reset link</button>
  {#if form?.sent}
    <p class="ok" role="status">
      If an account exists for that email, a reset link has been sent. Check your inbox
      (or http://localhost:8025 in dev).
    </p>
  {/if}
</form>

<p class="alt"><a href="/login">Back to sign in</a></p>

<style>
  h1 { font-size: var(--text-2xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); margin-bottom: var(--space-5); }
  form { display: grid; gap: var(--space-4); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
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
  .ok {
    padding: var(--space-3) var(--space-4);
    background: hsl(142 71% 35% / 0.12);
    color: hsl(142 71% 28%);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
  .alt a { color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px; }
  .alt { margin-top: var(--space-5); color: var(--color-fg-muted); font-size: var(--text-sm); text-align: center; }
</style>
