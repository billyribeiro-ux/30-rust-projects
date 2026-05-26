import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = process.env.E2E_BACKEND_URL ?? 'http://localhost:3009';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq(prefix: string): string {
  return `${prefix} ${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

async function createRecipe(
  request: import('@playwright/test').APIRequestContext,
  over: Partial<{ title: string; description: string; ingredients: string[]; instructions: string[] }> = {}
) {
  const title = over.title ?? uniq('E2E Recipe');
  const res = await request.post(`${BACKEND}/api/recipes`, {
    data: {
      title,
      description: over.description ?? 'A test recipe',
      ingredients: over.ingredients ?? ['2 apples', '200g flour', '1 tsp cinnamon'],
      instructions: over.instructions ?? ['Peel apples', 'Mix with flour', 'Bake at 200C for 45 min'],
      prep_minutes: 15,
      cook_minutes: 45,
      servings: 8
    }
  });
  return res.json();
}

test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});

test('create a recipe via the form, see it in the grid', async ({ page }) => {
  await gotoHydrated(page, '/recipes/new');
  const title = uniq('Form Recipe');
  await page.getByLabel(/^Title/i).fill(title);
  await page.getByLabel(/^Description/i).fill('Created via Playwright');
  await page.getByLabel(/^Ingredients/i).fill('eggs\nflour');
  await page.getByLabel(/^Instructions/i).fill('mix\nbake');
  await page.getByRole('button', { name: /Create/i }).click();
  // We land on the edit page after creating.
  await page.waitForURL(/\/recipes\/.+\/edit/);
  await expect(page.getByRole('heading', { level: 1, name: /Edit recipe/i })).toBeVisible();

  await gotoHydrated(page, '/');
  // The same RecipeCard snippet renders in two sections (main grid +
  // recently-added strip). Scope the assertion to the "All recipes"
  // region to keep the locator unambiguous.
  await expect(
    page.getByRole('region', { name: 'All recipes' }).getByRole('heading', { level: 2, name: title })
  ).toBeVisible();
});

test('recipe detail page emits valid Recipe JSON-LD', async ({ page, request }) => {
  const recipe = await createRecipe(request);
  await gotoHydrated(page, `/recipes/${recipe.slug}`);

  // Find the JSON-LD <script> by its id, read its text content, parse,
  // assert the shape.
  const scriptText = await page.locator('script#recipe-jsonld').textContent();
  expect(scriptText, 'JSON-LD script must be present').toBeTruthy();
  const ld = JSON.parse(scriptText ?? '{}');
  expect(ld['@context']).toBe('https://schema.org');
  expect(ld['@type']).toBe('Recipe');
  expect(ld.name).toBe(recipe.title);
  expect(Array.isArray(ld.recipeIngredient)).toBe(true);
  expect(ld.recipeIngredient.length).toBeGreaterThan(0);
  expect(Array.isArray(ld.recipeInstructions)).toBe(true);
  expect(ld.recipeInstructions[0]['@type']).toBe('HowToStep');
  expect(ld.recipeInstructions[0].position).toBe(1);
  expect(ld.prepTime).toMatch(/^PT\d+M$/);
  expect(ld.cookTime).toMatch(/^PT\d+M$/);
});

test('signed share link round-trip works, tampered link 404s', async ({ page, request }) => {
  const recipe = await createRecipe(request);
  // Mint a share link.
  const shareRes = await request.post(`${BACKEND}/api/recipes/${recipe.id}/share`);
  const share = await shareRes.json();
  expect(share.url).toMatch(/sig=.+&exp=\d+/);

  // Hitting the share page through the frontend should render the recipe.
  await gotoHydrated(page, share.url);
  await expect(page.getByRole('heading', { level: 1, name: recipe.title })).toBeVisible();

  // Tampering with the slug invalidates the signature → 404.
  const tampered = share.url.replace(`/share/${recipe.slug}`, '/share/other-slug');
  const res = await page.goto(tampered);
  expect(res?.status()).toBe(404);
});

test('service worker registers on the home page', async ({ page, browserName }) => {
  // Service workers only run on secure origins, but `localhost` is treated
  // as secure by Chromium so this works under the Playwright preview.
  test.skip(
    browserName !== 'chromium' && false,
    'service workers tested only in Chromium derivatives'
  );

  await gotoHydrated(page, '/');
  // First visit: the registration kicks off but `controller` may be null
  // until a reload.
  await page.reload();
  await page.waitForLoadState('networkidle');

  const registered = await page.evaluate(async () => {
    // Wait briefly for the SW to take control.
    for (let i = 0; i < 30; i++) {
      if (navigator.serviceWorker.controller) return true;
      await new Promise((r) => setTimeout(r, 100));
    }
    return Boolean(navigator.serviceWorker.controller);
  });
  expect(registered, 'navigator.serviceWorker.controller should be set after reload').toBe(true);
});

test('multipart upload of a non-image is rejected with 415', async ({ request }) => {
  const recipe = await createRecipe(request);
  // Send a multipart body whose `file` field is plain text — server-side
  // mime sniffing should reject before any disk write.
  const form = new FormData();
  form.append('file', new Blob(['hello world'], { type: 'image/jpeg' }), 'fake.jpg');
  const res = await request.post(`${BACKEND}/api/recipes/${recipe.id}/images`, {
    multipart: {
      file: {
        name: 'fake.jpg',
        mimeType: 'image/jpeg',
        buffer: Buffer.from('definitely not an image')
      }
    }
  });
  expect(res.status()).toBe(415);
});
