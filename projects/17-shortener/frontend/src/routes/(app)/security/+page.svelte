<script lang="ts">
  import { twoFAApi, ApiCallError } from '$lib/api';
  import type { TwoFASetup } from '$lib/types';
  import { ShieldCheck, QrCode, Key, Warning } from 'phosphor-svelte';

  // 2FA setup is purely client-side because we need to show one-time secrets
  // (QR + backup codes) that should never round-trip through SvelteKit form
  // actions or get persisted to logs.
  let setup: TwoFASetup | null = $state(null);
  let code = $state('');
  let backupCodes: string[] | null = $state(null);
  let error: string | null = $state(null);
  let pending = $state(false);

  async function startSetup() {
    error = null;
    pending = true;
    try {
      setup = await twoFAApi.setup(fetch);
      backupCodes = null;
    } catch (e) {
      error = e instanceof Error ? e.message : 'failed';
    } finally {
      pending = false;
    }
  }

  async function verifyCode() {
    error = null;
    pending = true;
    try {
      const res = await twoFAApi.verify(fetch, code.trim());
      backupCodes = res.backup_codes;
      setup = null;
      code = '';
    } catch (e) {
      error =
        e instanceof ApiCallError && e.fields?.[0]?.message
          ? e.fields[0].message
          : e instanceof Error
            ? e.message
            : 'failed';
    } finally {
      pending = false;
    }
  }

  async function disable2fa() {
    error = null;
    pending = true;
    try {
      await twoFAApi.disable(fetch, code.trim());
      backupCodes = null;
      setup = null;
      code = '';
      location.reload();
    } catch (e) {
      error = e instanceof Error ? e.message : 'failed';
    } finally {
      pending = false;
    }
  }
</script>

<svelte:head><title>Security</title></svelte:head>

<main>
  <header>
    <h1><ShieldCheck size="28" weight="duotone" /> Security</h1>
    <p class="lead">Two-factor authentication adds a one-time code to every sign-in.</p>
  </header>

  {#if error}
    <p class="error" role="alert"><Warning weight="bold" size="18" /> {error}</p>
  {/if}

  {#if backupCodes}
    <section class="panel" aria-label="Backup codes">
      <h2><Key size="20" weight="duotone" /> Save these backup codes</h2>
      <p class="warn">
        Each code works once. Store them somewhere safe (a password manager).
        We won't show them again.
      </p>
      <ul class="codes">
        {#each backupCodes as bc, i (i)}
          <li><code>{bc}</code></li>
        {/each}
      </ul>
      <p><a href="/">← Back to your links</a></p>
    </section>
  {:else if setup}
    <section class="panel" aria-label="2FA setup">
      <h2><QrCode size="20" weight="duotone" /> Step 1 — Scan with your authenticator</h2>
      <img src="data:image/png;base64,{setup.qr_png_base64}" alt="2FA QR code" class="qr" />
      <details>
        <summary>Or enter the secret manually</summary>
        <p><code class="uri">{setup.provisioning_uri}</code></p>
      </details>
      <h2>Step 2 — Confirm a code</h2>
      <label class="field">
        <span>6-digit code from your authenticator</span>
        <input
          type="text"
          inputmode="numeric"
          autocomplete="one-time-code"
          maxlength="6"
          bind:value={code}
          placeholder="123456"
        />
      </label>
      <button
        type="button"
        class="primary"
        onclick={verifyCode}
        disabled={pending || code.trim().length < 6}
      >
        Confirm & enable 2FA
      </button>
    </section>
  {:else}
    <section class="panel">
      <h2>Two-factor authentication</h2>
      <p>
        Once enabled, you'll need a 6-digit code from your authenticator
        (Authy / 1Password / Google Authenticator) to sign in.
      </p>
      <button type="button" class="primary" onclick={startSetup} disabled={pending}>
        Set up 2FA
      </button>

      <h2>Disable 2FA</h2>
      <p>If 2FA is already enabled, enter a current code (or backup code) to turn it off.</p>
      <label class="field">
        <span>Code</span>
        <input type="text" bind:value={code} maxlength="13" placeholder="123456" />
      </label>
      <button
        type="button"
        class="ghost"
        onclick={disable2fa}
        disabled={pending || code.trim().length < 4}
      >
        Disable 2FA
      </button>
    </section>
  {/if}
</main>

<style>
  main {
    max-width: 600px;
    margin-inline: auto;
    padding: var(--space-5) var(--space-4);
    display: grid;
    gap: var(--space-4);
  }
  h1 {
    font-size: var(--text-3xl);
    font-weight: 800;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  h2 { font-size: var(--text-lg); font-weight: 700; display: flex; align-items: center; gap: var(--space-2); }
  .lead { color: var(--color-fg-muted); }
  .panel {
    padding: var(--space-4);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    display: grid;
    gap: var(--space-3);
  }
  .field { display: grid; gap: var(--space-1); }
  .field > span { font-size: var(--text-sm); color: var(--color-fg-muted); }
  .field input {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--text-lg);
    letter-spacing: 0.1em;
  }
  .primary {
    padding: var(--space-2) var(--space-4);
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border-radius: var(--radius-sm);
    font-weight: 600;
    justify-self: start;
  }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .ghost {
    padding: var(--space-2) var(--space-3);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    justify-self: start;
  }
  .error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    color: var(--color-danger);
    background: hsl(0 72% 51% / 0.08);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-sm);
  }
  .warn {
    padding: var(--space-3);
    background: hsl(38 92% 45% / 0.12);
    border: 1px solid hsl(38 92% 45% / 0.3);
    border-radius: var(--radius-sm);
    color: hsl(38 92% 25%);
  }
  .qr {
    width: 220px;
    height: 220px;
    image-rendering: pixelated;
    background: white;
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    justify-self: center;
  }
  .uri {
    word-break: break-all;
    overflow-wrap: anywhere;
    font-size: var(--text-xs);
  }
  .codes {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-1);
  }
  .codes code {
    display: block;
    padding: var(--space-2);
    background: var(--color-bg);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono, ui-monospace, monospace);
    text-align: center;
  }
</style>
