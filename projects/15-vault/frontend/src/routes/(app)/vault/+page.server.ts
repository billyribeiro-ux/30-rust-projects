import { foldersApi, filesApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, url }) => {
  const folderId = url.searchParams.get('folder');
  const f = serverFetch(locals.sessionCookie);
  // Load all folders + the files for the current folder in parallel.
  const [folders, files] = await Promise.all([foldersApi.list(f), filesApi.list(f, folderId)]);
  return { folders, files, folderId };
};
