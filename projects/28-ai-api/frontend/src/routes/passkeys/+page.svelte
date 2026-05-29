<script lang="ts">
  let { } = $props();
</script>

<svelte:head><title>Passkeys — AI Inference</title></svelte:head>

<main>
  <a class="back" href="/keys">← API keys</a>
  <h1>Passkeys</h1>
  <p class="lead">
    The backend exposes the WebAuthn endpoints — <code>/api/passkeys/register/start</code>,
    <code>/register/finish</code>, <code>/login/start</code>, <code>/login/finish</code>.
    A production UI would call <code>navigator.credentials.create()</code> /
    <code>.get()</code> with the JSON the start endpoints return, then echo the
    result back to the finish endpoint.
  </p>
  <p>
    The integration test in <code>backend/tests/inference_flow.rs</code> exercises
    the password-auth + API-key + RPM-gating path. The passkey path needs a
    browser (or `webauthn-rs`'s test-fixture mode) which we don't ship in CI.
  </p>
</main>

<style>
  main { max-width: 720px; margin: 0 auto; padding: var(--space-6) var(--space-4); display: grid; gap: var(--space-3); }
  .back { color: var(--color-fg-muted); text-decoration: none; font-size: var(--text-sm); }
  h1 { font-size: var(--text-3xl); font-weight: 800; }
  .lead { color: var(--color-fg-muted); }
  code { background: var(--color-bg-sunken); padding: 0 var(--space-1); border-radius: var(--radius-sm); }
</style>
