<script lang="ts">
  import { enhance } from '$app/forms';
  import type { LayoutProps } from './$types';

  let { data, children }: LayoutProps = $props();
</script>

<div class="shell">
  <nav class="topnav" aria-label="Primary">
    <a class="brand" href="/boards">Kanban</a>
    <ul>
      <li><a href="/boards">Boards</a></li>
    </ul>
    <div class="right">
      <span class="who" title={data.user?.email}>{data.user?.name || data.user?.email}</span>
      <form method="POST" action="/logout" use:enhance>
        <button type="submit" class="ghost">Sign out</button>
      </form>
    </div>
  </nav>

  <main>{@render children()}</main>
</div>

<style>
  .shell {
    min-height: 100dvh;
    display: grid;
    grid-template-rows: auto 1fr;
  }
  .topnav {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border-bottom: 1px solid var(--color-border);
  }
  .brand {
    font-weight: 800;
    color: var(--color-accent);
  }
  .topnav ul {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    list-style: none;
    padding: 0;
    margin: 0;
    flex: 1;
  }
  .topnav ul a {
    color: var(--color-fg-muted);
    text-decoration: none;
    font-size: var(--text-sm);
  }
  .topnav ul a:hover {
    color: var(--color-fg);
  }
  .right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .who {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .ghost {
    padding: var(--space-1) var(--space-3);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .ghost:hover {
    border-color: var(--color-border-strong);
  }
</style>
