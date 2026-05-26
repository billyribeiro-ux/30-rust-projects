import { test, expect, type Page } from '@playwright/test';

// Visual regression: render a workout with a known PR set and snapshot
// the PR badge. The seed is deterministic — same exercise name, same
// weight progression — so the rendered PR row is byte-identical across
// runs (modulo font subpixel differences, which Playwright's tolerance
// absorbs by default).

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test('PR badge matches the committed snapshot', async ({ page, request }) => {
  // The snapshot is of the BADGE COMPONENT (deterministic CSS, no
  // data-driven content), so the seed need only PRODUCE a PR — it does
  // not need to be deterministic. We use a unique exercise per run so
  // the very-first-set-always-PR property holds even when the DB has
  // accumulated history from previous runs.
  const viewportName = test.info().project.name;
  const stamp = `${Date.now()}-${Math.floor(Math.random() * 100000)}`;
  const exName = `Visual Bench ${viewportName} ${stamp}`;
  const exRes = await request.post('http://localhost:3008/api/exercises', {
    data: { name: exName, muscle_group: 'chest' }
  });
  expect(exRes.ok(), `exercise create failed: ${await exRes.text()}`).toBe(true);
  const ex = (await exRes.json()) as { id: string };

  const wRes = await request.post('http://localhost:3008/api/workouts', {
    data: {
      name: `Visual workout ${stamp}`,
      sets: [{ exercise_id: ex.id, weight_minor: 100_000, reps: 5, rir: 0 }]
    }
  });
  expect(wRes.ok(), `workout create failed: ${await wRes.text()}`).toBe(true);
  const w = (await wRes.json()) as {
    id: string;
    sets: { id: string; is_pr: boolean }[];
  };
  expect(w.sets[0]?.is_pr).toBe(true);

  await gotoHydrated(page, `/workouts/${w.id}`);
  const badge = page.getByRole('img', { name: 'Personal record' }).first();
  await expect(badge).toBeVisible();
  await expect(badge).toHaveScreenshot('pr-badge.png', {
    maxDiffPixelRatio: 0.02
  });
});
