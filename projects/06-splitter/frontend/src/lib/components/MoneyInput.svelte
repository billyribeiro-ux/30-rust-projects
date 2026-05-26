<script lang="ts">
  import { parseMoney, formatCents } from '$lib/money';

  type Props = {
    /**
     * The bindable cents value. The parent can `bind:cents={...}` to get a
     * two-way binding. The `$bindable()` rune means the parent doesn't need
     * to pass a prop — but if they do, they get to read AND write.
     */
    cents?: number | null;
    label: string;
    name?: string;
    placeholder?: string;
    /** Show an error message under the input. */
    error?: string;
    required?: boolean;
    /** Optional id for label-input association (otherwise auto-generated). */
    id?: string;
  };

  let {
    cents = $bindable(null),
    label,
    name,
    placeholder = '0.00',
    error,
    required = false,
    id: idProp
  }: Props = $props();

  const autoId = $props.id();
  const id = $derived(idProp ?? `money-${autoId}`);

  // The text the user sees. Initialized from the cents value, updated on each
  // keystroke, and validated on blur. We keep a separate text state so the
  // user can type "12." without us spuriously formatting back to "12.00".
  let text = $state(cents === null || cents === undefined ? '' : formatCents(cents));
  let touched = $state(false);

  function onInput(e: Event) {
    text = (e.target as HTMLInputElement).value;
    const parsed = parseMoney(text);
    cents = parsed;
  }

  function onBlur() {
    touched = true;
    // Pretty-print on blur: "12" → "12.00", but only if it parsed cleanly.
    if (cents !== null && cents !== undefined) {
      text = formatCents(cents);
    }
  }

  const showError = $derived(error || (touched && text.trim().length > 0 && cents === null));
  const errorText = $derived(
    error ?? (touched && text.trim().length > 0 && cents === null ? 'Enter a valid amount' : '')
  );
</script>

<div class="field" class:has-error={!!showError}>
  <label for={id}>{label}</label>
  <div class="input-wrap">
    <span class="prefix" aria-hidden="true">$</span>
    <input
      {id}
      type="text"
      inputmode="decimal"
      {name}
      {placeholder}
      {required}
      value={text}
      oninput={onInput}
      onblur={onBlur}
      aria-invalid={showError ? 'true' : undefined}
      aria-describedby={showError ? `${id}-error` : undefined}
    />
  </div>
  {#if showError}
    <p class="error" id="{id}-error" role="alert">{errorText}</p>
  {/if}
</div>

<style>
  .field {
    display: grid;
    gap: var(--space-1);
  }

  label {
    font-size: var(--text-sm);
    color: var(--color-fg-muted);
  }

  .input-wrap {
    position: relative;
  }

  .prefix {
    position: absolute;
    left: var(--space-3);
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-fg-muted);
    font-weight: 500;
  }

  input {
    width: 100%;
    padding: var(--space-3) var(--space-4) var(--space-3) calc(var(--space-3) + 12px);
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-base);
    font-variant-numeric: tabular-nums;
  }

  input:focus-visible {
    border-color: var(--color-accent);
    outline-offset: 0;
  }

  .field.has-error input {
    border-color: var(--color-danger);
  }

  .error {
    color: var(--color-danger);
    font-size: var(--text-xs);
  }
</style>
