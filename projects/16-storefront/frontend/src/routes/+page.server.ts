import type { PageServerLoad } from './$types';
import { productsApi } from '$lib/api';
import type { Product } from '$lib/types';

/**
 * Storefront landing page: SSR-load the public product list so the first
 * paint shows real data. Falls back to an empty list if the backend is
 * unreachable (e.g. during initial setup or a Postgres-less E2E mock).
 */
export const load: PageServerLoad = async ({ fetch }) => {
  let products: Product[] = [];
  let error: string | null = null;
  try {
    products = await productsApi.list(fetch);
  } catch (e) {
    error = e instanceof Error ? e.message : 'failed to load products';
  }
  return { products, error };
};
