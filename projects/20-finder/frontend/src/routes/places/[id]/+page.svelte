<script lang="ts">
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  // svelte-ignore state_referenced_locally
  const place = data.place;

  const jsonLd = $derived(
    JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'Restaurant',
      name: place.name,
      address: place.address,
      servesCuisine: place.cuisine,
      geo: { '@type': 'GeoCoordinates', latitude: place.lat, longitude: place.lng },
      aggregateRating:
        place.avg_rating !== null
          ? {
              '@type': 'AggregateRating',
              ratingValue: place.avg_rating,
              reviewCount: place.review_count
            }
          : undefined,
      review: place.reviews.map((r) => ({
        '@type': 'Review',
        reviewRating: { '@type': 'Rating', ratingValue: r.rating, bestRating: 5 },
        author: { '@type': 'Person', name: r.user_name },
        reviewBody: r.body,
        datePublished: r.created_at
      }))
    })
  );
</script>

<svelte:head>
  <title>{place.name} — Finder</title>
  <meta name="description" content={`${place.cuisine} restaurant at ${place.address}.`} />
  {@html `<script type="application/ld+json">${jsonLd}</script>`}
</svelte:head>

<header>
  <a href="/" class="back">← Back to search</a>
  <h1>{place.name}</h1>
  <p class="cuisine">{place.cuisine}</p>
  <p class="address">{place.address}</p>
  {#if place.distance_m !== null}
    <p class="meta">{Math.round(place.distance_m)} m away</p>
  {/if}
  {#if place.avg_rating !== null}
    <p class="meta">⭐ {place.avg_rating.toFixed(1)} ({place.review_count} reviews)</p>
  {/if}
</header>

<section aria-labelledby="reviews-h">
  <h2 id="reviews-h">Reviews</h2>
  {#if place.reviews.length === 0}
    <p class="empty">No reviews yet.</p>
  {:else}
    <ul>
      {#each place.reviews as r (r.id)}
        <li>
          <p class="rating" aria-label={`Rated ${r.rating} of 5`}>
            {'★'.repeat(r.rating)}{'☆'.repeat(5 - r.rating)}
          </p>
          <p class="body">{r.body}</p>
          <p class="byline">— {r.user_name}, <time datetime={r.created_at}>{new Date(r.created_at).toLocaleDateString()}</time></p>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  header,
  section {
    max-width: 720px;
    margin: 0 auto;
    padding: var(--space-5) var(--space-4);
  }
  .back {
    color: var(--color-fg-muted);
    text-decoration: none;
    font-size: var(--text-sm);
  }
  h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
    margin-top: var(--space-2);
  }
  .cuisine {
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: var(--text-sm);
  }
  .address,
  .meta {
    color: var(--color-fg-muted);
  }
  h2 {
    font-size: var(--text-xl);
    font-weight: 700;
    margin-bottom: var(--space-3);
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: var(--space-3);
  }
  li {
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .rating {
    color: var(--color-accent);
  }
  .body {
    margin-top: var(--space-1);
  }
  .byline {
    margin-top: var(--space-2);
    color: var(--color-fg-muted);
    font-size: var(--text-sm);
  }
  .empty {
    color: var(--color-fg-muted);
  }
</style>
