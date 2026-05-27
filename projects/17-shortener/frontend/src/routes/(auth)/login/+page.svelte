<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();

  let submitting = $state(false);

  // When 2FA is on, stage 1 returns `requires_2fa + intermediate`. We render
  // the code-entry form instead of the email/password fields.
  const needs2fa = $derived(
    !!form && (form as { requires_2fa?: boolean }).requires_2fa === true
  );
</script>

<svelte:head><title>Sign in</title></svelte:head>

<h1>Sign in</h1>
<p class="lead">Welcome back.</p>

{#if needs2fa}
  <form
    method="POST"
    action="?/twoFactor"
    use:enhance={() => {
      submitting = true;
      return async ({ update }) => {
        submitting = false;
        await update();
      };
    }}
  >
    <input type="hidden" name="intermediate" value={(form as { intermediate?: string }).intermediate ?? ''} />
    <input type="hidden" name="email" value={(form as { email?: string }).email ?? ''} />
    <label class="field">
      <span>6-digit code (or backup code)</span>
      <input
        type="text"
        name="code"
        required
        autocomplete="one-time-code"
        inputmode="numeric"
        maxlength="13"
        placeholder="123456"
      />
    </label>
    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error}</p>
    {/if}
    <button type="submit" class="primary" disabled={submitting}>
      {submitting ? 'Verifying…' : 'Verify'}
    </button>
  </form>
{:else}
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
      <span>Email</span>
      <input
        type="email"
        name="email"
        required
        autocomplete="email"
        value={(form?.email as string | undefined) ?? ''}
      />
    </label>
    <label class="field">
      <span>Password</span>
      <input type="password" name="password" required autocomplete="current-password" minlength="12" />
    </label>

    {#if form && 'error' in form && form.error}
      <p class="error" role="alert">{form.error}</p>
    {/if}

    <button type="submit" class="primary" disabled={submitting}>
      {submitting ? 'Signing in…' : 'Sign in'}
    </button>
  </form>

  <p class="alt">
    Need an account? <a href="/register">Create one</a><br />
    Forgot your password? <a href="/forgot">Reset it</a>
  </p>
{/if}

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
  .alt {
    margin-top: var(--space-5);
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    text-align: center;
    line-height: var(--leading-relaxed);
  }
  .alt a { color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px; }
</style>
