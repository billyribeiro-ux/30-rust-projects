<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();

  let submitting = $state(false);
  let joining = $state(false);
</script>

<svelte:head><title>Rooms — Chat</title></svelte:head>

<main>
  <header>
    <h1>Rooms</h1>
    <p class="lead">Hi, {data.user?.name || data.user?.email}.</p>
  </header>

  <section class="panel" aria-label="Your rooms">
    <h2>Your rooms</h2>
    {#if data.rooms.length === 0}
      <p class="empty">You haven't joined any rooms yet. Create one below or join an existing one by slug.</p>
    {:else}
      <ul class="rooms">
        {#each data.rooms as r (r.id)}
          <li>
            <a href="/rooms/{r.slug}">
              <span class="slug">#{r.slug}</span>
              <span class="name">{r.name}</span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="panel" aria-label="Create a room">
    <h2>Create a room</h2>
    <form
      method="POST"
      action="?/create"
      use:enhance={() => {
        submitting = true;
        return async ({ update }) => {
          submitting = false;
          await update();
        };
      }}
    >
      <label class="field">
        <span>Slug <em>*</em></span>
        <input type="text" name="slug" required pattern="[a-z0-9][a-z0-9-]{'{'}0,63{'}'}" placeholder="general"
               value={(form?.slug as string | undefined) ?? ''} />
      </label>
      <label class="field">
        <span>Name <em>*</em></span>
        <input type="text" name="name" required maxlength="200" placeholder="General chat"
               value={(form?.name as string | undefined) ?? ''} />
      </label>
      {#if form && 'error' in form && form.error}
        <p class="error" role="alert">{form.error}</p>
      {/if}
      <button type="submit" class="primary" disabled={submitting}>
        {submitting ? 'Creating…' : 'Create room'}
      </button>
    </form>
  </section>

  <section class="panel" aria-label="Join a room">
    <h2>Join an existing room</h2>
    <form
      method="POST"
      action="?/join"
      use:enhance={() => {
        joining = true;
        return async ({ update }) => {
          joining = false;
          await update();
        };
      }}
    >
      <label class="field">
        <span>Slug</span>
        <input type="text" name="slug" required pattern="[a-z0-9][a-z0-9-]{'{'}0,63{'}'}"
               placeholder="general"
               value={(form?.joinSlug as string | undefined) ?? ''} />
      </label>
      {#if form && 'joinError' in form && form.joinError}
        <p class="error" role="alert">{form.joinError}</p>
      {/if}
      <button type="submit" class="ghost" disabled={joining}>
        {joining ? 'Joining…' : 'Join'}
      </button>
    </form>
  </section>
</main>

<style>
  main { max-width: var(--container-md); margin-inline: auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-5); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }
  .panel { padding: var(--space-5); background: var(--color-bg-elev); border: 1px solid var(--color-border); border-radius: var(--radius-md); display: grid; gap: var(--space-3); }
  h2 { font-size: var(--text-lg); font-weight: 700; }
  .rooms { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-2); }
  .rooms a {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-3);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
    color: var(--color-fg);
    text-decoration: none;
  }
  .rooms a:hover { background: var(--color-bg-sunken); }
  .slug { color: var(--color-accent); font-weight: 600; font-family: var(--font-mono, monospace); font-size: var(--text-sm); }
  .name { color: var(--color-fg-muted); font-size: var(--text-sm); }
  form { display: grid; gap: var(--space-3); }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field em { color: var(--color-danger); font-style: normal; }
  .field input {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-family: var(--font-mono, monospace);
    font-size: var(--text-sm);
  }
  .primary {
    padding: var(--space-2) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-md);
    font-weight: 600;
    justify-self: start;
  }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .ghost {
    padding: var(--space-2) var(--space-4);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    justify-self: start;
  }
  .empty { color: var(--color-fg-muted); padding: var(--space-3); background: var(--color-bg-sunken); border-radius: var(--radius-sm); text-align: center; }
  .error {
    padding: var(--space-3);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
</style>
