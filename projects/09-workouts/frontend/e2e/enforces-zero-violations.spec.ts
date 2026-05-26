import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// This file's name is load-bearing: it is the CI GATE. A single violation
// here fails the build; this is not advisory, this is "do not merge".
// We hit every public route and run the full WCAG 2 A + AA ruleset.

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

const routes = ['/', '/exercises', '/workouts/new'];

for (const route of routes) {
  test(`axe-core: ${route} has zero WCAG 2 A/AA violations`, async ({ page }) => {
    await gotoHydrated(page, route);
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(
      results.violations,
      JSON.stringify(results.violations, null, 2)
    ).toEqual([]);
  });
}
