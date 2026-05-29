<script lang="ts">
  import type { PageProps } from './$types';
  import type { PlaceRow } from '$lib/types';
  import { authApi, backendUrl } from '$lib/api';

  let { data }: PageProps = $props();

  // Seed once from SSR data — the form posts via GET so a new page load
  // brings fresh `data.q`/`data.cuisine`; we don't need them to react in-place.
  // svelte-ignore state_referenced_locally
  let q = $state(data.q);
  // svelte-ignore state_referenced_locally
  let cuisine = $state(data.cuisine);
  let geoStatus = $state<'idle' | 'asking' | 'denied' | 'ok'>('idle');

  // Restaurant JSON-LD for the result set — emitted server-side so
  // crawlers see structured data without JS.
  const jsonLd = $derived(
    JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'ItemList',
      itemListElement: (data.places as PlaceRow[]).slice(0, 20).map((p, i) => ({
        '@type': 'ListItem',
        position: i + 1,
        item: {
          '@type': 'Restaurant',
          name: p.name,
          address: p.address,
          servesCuisine: p.cuisine,
          geo: { '@type': 'GeoCoordinates', latitude: p.lat, longitude: p.lng },
          aggregateRating: p.avg_rating
            ? {
                '@type': 'AggregateRating',
                ratingValue: p.avg_rating,
                reviewCount: p.review_count
              }
            : undefined
        }
      }))
    })
  );

  function askLocation() {
    if (typeof navigator === 'undefined' || !navigator.geolocation) {
      geoStatus = 'denied';
      return;
    }
    geoStatus = 'asking';
    navigator.geolocation.getCurrentPosition(
      (pos) => {
        geoStatus = 'ok';
        const u = new URL(window.location.href);
        u.searchParams.set('lat', pos.coords.latitude.toFixed(6));
        u.searchParams.set('lng', pos.coords.longitude.toFixed(6));
        window.location.assign(u.toString());
      },
      () => {
        geoStatus = 'denied';
      },
      { timeout: 8000 }
    );
  }
</script>

<svelte:head>
  {@html `<script type="application/ld+json">${jsonLd}</script>`}
</svelte:head>

<nav class="topnav" aria-label="Primary">
  <a class="brand" href="/">Finder</a>
  <div class="right">
    {#if data.user}
      <span class="who">{data.user.name || data.user.email}</span>
      <a class="ghost" href="/logout">Sign out</a>
    {:else}
      <a class="ghost" href={authApi.oauthStart('google')} data-sveltekit-reload>Google</a>
      <a class="ghost" href={authApi.oauthStart('github')} data-sveltekit-reload>GitHub</a>
      <a class="ghost" href="/login">Email</a>
    {/if}
  </div>
</nav>

<main>
  <section class="hero">
    <h1>Find a place</h1>
    <p class="lead">Search nearby restaurants by cuisine and distance.</p>

    <form method="GET" class="searchbar">
      <input
        type="search"
        name="q"
        placeholder="Search by name or address"
        bind:value={q}
        aria-label="Search query"
      />
      <input
        type="text"
        name="cuisine"
        placeholder="Cuisine (e.g. thai)"
        bind:value={cuisine}
        aria-label="Cuisine"
      />
      {#if data.lat}<input type="hidden" name="lat" value={data.lat} />{/if}
      {#if data.lng}<input type="hidden" name="lng" value={data.lng} />{/if}
      <button type="submit" class="primary">Search</button>
    </form>

    <div class="geo">
      {#if geoStatus === 'idle' || geoStatus === 'asking'}
        <button type="button" class="ghost" onclick={askLocation} disabled={geoStatus === 'asking'}>
          {geoStatus === 'asking' ? 'Locating…' : 'Use my location'}
        </button>
      {:else if geoStatus === 'denied'}
        <p class="hint" role="alert">
          Location unavailable. <a href="?lat=40.7&lng=-74">Try NYC</a> or
          <a href="?lat=37.77&lng=-122.41">SF</a>.
        </p>
      {:else if data.lat && data.lng}
        <p class="hint">Searching within 2 km of ({data.lat}, {data.lng})</p>
      {/if}
    </div>
  </section>

  <section class="results" aria-label="Search results">
    {#if data.error}
      <p class="error" role="alert">{data.error}</p>
    {:else if data.places.length === 0}
      <p class="empty">No places yet. Backend seed is empty — see COMMANDS §3 to insert sample rows.</p>
    {:else}
      <ul>
        {#each data.places as place (place.id)}
          <li>
            <a class="card" href={`/places/${place.id}${data.lat ? `?lat=${data.lat}&lng=${data.lng}` : ''}`}>
              <span class="name">{place.name}</span>
              <span class="cuisine">{place.cuisine}</span>
              <span class="address">{place.address}</span>
              <span class="meta">
                {#if place.distance_m !== null}
                  <strong>{Math.round(place.distance_m)} m</strong>
                {/if}
                {#if place.avg_rating !== null}
                  · ⭐ {place.avg_rating.toFixed(1)} ({place.review_count})
                {/if}
              </span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <p class="info">
    Backend at <code>{backendUrl()}</code>. OAuth providers must be configured in
    <code>backend/.env</code> for the Google/GitHub buttons to redirect successfully.
  </p>
</main>

<style>
  .topnav {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border-bottom: 1px solid var(--color-border);
  }
  .brand {
    font-weight: 800;
    color: var(--color-accent);
    flex: 1;
  }
  .right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .ghost {
    padding: var(--space-1) var(--space-3);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    text-decoration: none;
  }
  .who {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  main {
    max-width: 960px;
    margin: 0 auto;
    padding: var(--space-6) var(--space-4);
    display: grid;
    gap: var(--space-6);
  }
  .hero h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
  }
  .lead {
    color: var(--color-fg-muted);
    margin-bottom: var(--space-4);
  }
  .searchbar {
    display: grid;
    grid-template-columns: 2fr 1fr auto;
    gap: var(--space-2);
  }
  .searchbar input {
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
  .geo {
    margin-top: var(--space-2);
  }
  .hint {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .results ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-3);
  }
  .card {
    display: grid;
    gap: var(--space-1);
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    text-decoration: none;
    color: inherit;
  }
  .name {
    font-weight: 600;
  }
  .cuisine {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .address {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .meta {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .empty,
  .info {
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
    text-align: center;
  }
  .error {
    padding: var(--space-3) var(--space-4);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-md);
  }

  @media (max-width: 640px) {
    .searchbar {
      grid-template-columns: 1fr;
    }
  }
</style>
