<script lang="ts">
  import { invalidateAll, goto } from '$app/navigation';
  import { foldersApi, filesApi } from '$lib/api';
  import { uploadFile, type UploadProgress } from '$lib/upload-client';
  import type { Folder, FileRow } from '$lib/types';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  // We capture initial data into local state so the dropzone progress UI
  // can render without bouncing values back through SvelteKit.
  // svelte-ignore state_referenced_locally
  let folders = $state<Folder[]>(data.folders);
  // svelte-ignore state_referenced_locally
  let files = $state<FileRow[]>(data.files);
  // svelte-ignore state_referenced_locally
  let folderId = $state<string | null>(data.folderId);

  $effect(() => {
    folders = data.folders;
    files = data.files;
    folderId = data.folderId;
  });

  type UploadingItem = {
    name: string;
    size: number;
    loaded: number;
    percent: number;
    error?: string;
  };
  let uploading = $state<UploadingItem[]>([]);

  // ---------- breadcrumb ----------
  // Walk up `parent_id` chain from the current folder. The root is null,
  // which we render as "All files".
  const breadcrumb = $derived.by(() => {
    const trail: Folder[] = [];
    let cur: string | null = folderId;
    let guard = 0;
    while (cur && guard < 64) {
      const f = folders.find((x) => x.id === cur);
      if (!f) break;
      trail.unshift(f);
      cur = f.parent_id;
      guard += 1;
    }
    return trail;
  });

  const childFolders = $derived(folders.filter((f) => f.parent_id === folderId));

  // ---------- new folder ----------
  let newFolderName = $state('');
  let creating = $state(false);
  async function createFolder() {
    const name = newFolderName.trim();
    if (!name) return;
    creating = true;
    try {
      await foldersApi.create(fetch, name, folderId);
      newFolderName = '';
      await invalidateAll();
    } catch (e) {
      console.error(e);
    } finally {
      creating = false;
    }
  }

  // ---------- delete ----------
  async function removeFile(id: string) {
    if (!confirm('Delete this file?')) return;
    await filesApi.remove(fetch, id);
    await invalidateAll();
  }

  // ---------- upload ----------
  async function handleFiles(list: FileList | File[]) {
    const arr = Array.from(list);
    for (const file of arr) {
      const item: UploadingItem = {
        name: file.name,
        size: file.size,
        loaded: 0,
        percent: 0
      };
      uploading = [...uploading, item];
      try {
        await uploadFile(file, folderId, {
          onProgress: (p: UploadProgress) => {
            item.loaded = p.loaded;
            item.percent = p.percent;
            // Trigger reactivity by replacing the array entry.
            uploading = uploading.map((u) => (u === item ? { ...item } : u));
          }
        });
      } catch (e) {
        item.error = (e as Error).message;
        uploading = uploading.map((u) => (u === item ? { ...item } : u));
      }
    }
    await invalidateAll();
    // Strip completed-without-error items after a beat.
    setTimeout(() => {
      uploading = uploading.filter((u) => u.error || u.percent < 1);
    }, 1500);
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer?.files) {
      void handleFiles(e.dataTransfer.files);
    }
  }
  function onDragOver(e: DragEvent) {
    e.preventDefault();
  }
  function onFilePicked(e: Event) {
    const t = e.target as HTMLInputElement;
    if (t.files) {
      void handleFiles(t.files);
      t.value = '';
    }
  }

  function fmtSize(b: number): string {
    if (b < 1024) return `${b} B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
    if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(1)} MB`;
    return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function navigateTo(id: string | null) {
    const u = new URL(window.location.href);
    if (id) u.searchParams.set('folder', id);
    else u.searchParams.delete('folder');
    void goto(u.pathname + (u.search ? u.search : ''), { invalidateAll: true });
  }
</script>

<svelte:head><title>Vault — Files</title></svelte:head>

<section class="vault">
  <header>
    <nav aria-label="Breadcrumb" class="crumbs">
      <ol>
        <li>
          <button class="crumb" onclick={() => navigateTo(null)} type="button">All files</button>
        </li>
        {#each breadcrumb as f (f.id)}
          <li aria-hidden="true" class="sep">/</li>
          <li><button class="crumb" onclick={() => navigateTo(f.id)} type="button">{f.name}</button></li>
        {/each}
      </ol>
    </nav>

    <form class="new-folder" onsubmit={(e) => { e.preventDefault(); void createFolder(); }}>
      <label class="visually-hidden" for="new-folder-name">New folder name</label>
      <input
        id="new-folder-name"
        type="text"
        placeholder="New folder name"
        bind:value={newFolderName}
        maxlength="200"
      />
      <button type="submit" class="ghost" disabled={creating || !newFolderName.trim()}>
        New folder
      </button>
    </form>
  </header>

  <!-- Dropzone -->
  <div
    class="dropzone"
    ondrop={onDrop}
    ondragover={onDragOver}
    role="region"
    aria-label="Drop files here to upload"
  >
    <p>Drop files here, or</p>
    <label class="picker primary">
      <input type="file" multiple onchange={onFilePicked} />
      Choose files
    </label>
  </div>

  {#if uploading.length > 0}
    <ul class="uploads" aria-label="Active uploads">
      {#each uploading as u (u.name + u.size)}
        <li class:err={!!u.error}>
          <div class="row">
            <span class="name">{u.name}</span>
            <span class="size">{fmtSize(u.loaded)} / {fmtSize(u.size)}</span>
          </div>
          <progress max="1" value={u.percent}></progress>
          {#if u.error}<p class="err-msg" role="alert">{u.error}</p>{/if}
        </li>
      {/each}
    </ul>
  {/if}

  <h2>Folders</h2>
  {#if childFolders.length === 0}
    <p class="empty">No subfolders here.</p>
  {:else}
    <ul class="folders" aria-label="Folders">
      {#each childFolders as f (f.id)}
        <li>
          <button class="folder" onclick={() => navigateTo(f.id)} type="button">
            <span aria-hidden="true">📁</span>
            <span>{f.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <h2>Files</h2>
  {#if files.length === 0}
    <p class="empty">No files in this folder yet.</p>
  {:else}
    <table class="files">
      <thead>
        <tr><th>Name</th><th>Size</th><th>Type</th><th><span class="visually-hidden">Actions</span></th></tr>
      </thead>
      <tbody>
        {#each files as f (f.id)}
          <tr>
            <td><a href={filesApi.downloadUrl(f.id)}>{f.name}</a></td>
            <td>{fmtSize(f.size)}</td>
            <td>{f.mime}</td>
            <td><button class="danger" onclick={() => removeFile(f.id)} type="button">Delete</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .vault { padding: var(--space-4); display: grid; gap: var(--space-4); max-width: 960px; margin: 0 auto; width: 100%; }
  header { display: grid; gap: var(--space-3); }
  @media (min-width: 768px) {
    header { display: flex; justify-content: space-between; align-items: center; }
  }
  .crumbs ol { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-1); list-style: none; padding: 0; margin: 0; }
  .crumb { color: var(--color-accent); text-decoration: underline; text-underline-offset: 2px; font-size: var(--text-sm); }
  .sep { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .new-folder { display: flex; gap: var(--space-2); }
  .new-folder input {
    padding: var(--space-2) var(--space-3);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .ghost {
    padding: var(--space-2) var(--space-3);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }
  .ghost:disabled { opacity: 0.5; cursor: not-allowed; }
  .dropzone {
    border: 2px dashed var(--color-border-strong);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    background: var(--color-bg-sunken);
    display: grid;
    place-items: center;
    gap: var(--space-3);
    text-align: center;
  }
  .picker { display: inline-block; padding: var(--space-3) var(--space-4); border-radius: var(--radius-md); cursor: pointer; }
  .primary { background: var(--color-accent); color: var(--color-accent-fg); font-weight: 600; }
  .picker input { display: none; }
  .uploads { list-style: none; padding: 0; display: grid; gap: var(--space-2); }
  .uploads li {
    background: var(--color-bg-elev);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
  }
  .uploads li.err { border-color: var(--color-danger); }
  .row { display: flex; justify-content: space-between; font-size: var(--text-sm); margin-bottom: var(--space-1); }
  .name { font-weight: 600; }
  .size { color: var(--color-fg-muted); font-variant-numeric: tabular-nums; }
  progress { width: 100%; height: 8px; }
  .err-msg { color: var(--color-danger); font-size: var(--text-sm); margin-top: var(--space-1); }
  h2 { font-size: var(--text-lg); font-weight: 700; margin-top: var(--space-2); }
  .empty { color: var(--color-fg-muted); font-size: var(--text-sm); }
  .folders { list-style: none; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: var(--space-2); }
  .folder {
    display: flex; gap: var(--space-2); align-items: center;
    padding: var(--space-3); width: 100%;
    background: var(--color-bg-elev); border: 1px solid var(--color-border);
    border-radius: var(--radius-md); text-align: left;
    color: var(--color-fg);
  }
  .folder:hover { border-color: var(--color-border-strong); }
  .files { width: 100%; border-collapse: collapse; font-size: var(--text-sm); }
  .files th, .files td { padding: var(--space-2) var(--space-3); text-align: left; border-bottom: 1px solid var(--color-border); }
  .files th { color: var(--color-fg-muted); font-weight: 600; font-size: var(--text-xs); text-transform: uppercase; }
  .files td a { color: var(--color-accent); text-decoration: underline; }
  .danger {
    padding: var(--space-1) var(--space-2);
    background: hsl(0 72% 51% / 0.08);
    color: var(--color-danger);
    border: 1px solid hsl(0 72% 51% / 0.3);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
  }
  .visually-hidden {
    position: absolute; width: 1px; height: 1px;
    padding: 0; margin: -1px; overflow: hidden;
    clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;
  }
</style>
