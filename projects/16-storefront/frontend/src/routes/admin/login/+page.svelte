<script lang="ts">
  import { authApi, ApiCallError } from '$lib/api';
  import { goto } from '$app/navigation';

  let email = $state('');
  let password = $state('');
  let mode = $state<'login' | 'register'>('login');
  let name = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function submit(event: Event) {
    event.preventDefault();
    error = null;
    busy = true;
    try {
      if (mode === 'login') {
        await authApi.login(window.fetch.bind(window), email, password);
      } else {
        await authApi.register(window.fetch.bind(window), email, password, name);
      }
      await goto('/admin');
    } catch (e) {
      error = e instanceof ApiCallError ? e.message : 'authentication failed';
      busy = false;
    }
  }
</script>

<main class="auth">
  <h1>Admin {mode === 'login' ? 'sign in' : 'register'}</h1>
  <form onsubmit={submit}>
    <label for="email">Email</label>
    <input
      id="email"
      type="email"
      bind:value={email}
      autocomplete="email"
      required
    />

    {#if mode === 'register'}
      <label for="name">Name</label>
      <input id="name" type="text" bind:value={name} />
    {/if}

    <label for="password">Password</label>
    <input
      id="password"
      type="password"
      bind:value={password}
      autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
      minlength={12}
      required
    />

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <button type="submit" class="primary" disabled={busy}>
      {busy ? 'Working…' : mode === 'login' ? 'Sign in' : 'Create account'}
    </button>

    <button
      type="button"
      class="ghost"
      onclick={() => {
        mode = mode === 'login' ? 'register' : 'login';
        error = null;
      }}
    >
      {mode === 'login' ? 'Create an admin account' : 'Already have one? Sign in'}
    </button>
  </form>
</main>

<style>
  .auth {
    max-width: 26rem;
    margin: var(--space-8) auto;
    padding: 0 var(--space-4);
  }
  form {
    display: grid;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }
  input {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
  .primary {
    background: var(--color-accent);
    color: var(--color-accent-fg);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font-weight: 600;
    margin-top: var(--space-2);
  }
  .primary[disabled] {
    opacity: 0.6;
    cursor: progress;
  }
  .ghost {
    color: var(--color-accent);
    text-align: center;
    padding: var(--space-2);
  }
  .error {
    color: var(--color-danger);
  }
</style>
