<script lang="ts">
  import '../app.css';

  let { children } = $props();
</script>

<svelte:head>
  <title>TODO Manager</title>
  <meta
    name="description"
    content="A tiny, fast, distraction-free TODO app. Add tasks, check them off, get them out of your head."
  />
</svelte:head>

{@render children()}
