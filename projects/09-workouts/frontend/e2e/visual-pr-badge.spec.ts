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
  // Deterministic seed name keyed by viewport so parallel runs don't race.
  const viewportName = test.info().project.name;
  const exName = `Visual Bench (${viewportName})`;
  // Re-create the exercise (ignore conflict — name is unique).
  await request.post('http://localhost:3008/api/exercises', {
    data: { name: exName, muscle_group: 'chest' }
  });
  const exList = (await request
    .get('http://localhost:3008/api/exercises?q=' + encodeURIComponent('Visual'))
    .then((r) => r.json())) as { id: string; name: string }[];
  const ex = exList.find((e) => e.name === exName);
  if (!ex) throw new Error('seed exercise missing');

  // One PR-able set in a fresh workout. The first set is always a PR.
  const w = (await request
    .post('http://localhost:3008/api/workouts', {
      data: {
        name: `Visual workout (${viewportName})`,
        sets: [{ exercise_id: ex.id, weight_minor: 100_000, reps: 5, rir: 0 }]
      }
    })
    .then((r) => r.json())) as { id: string; sets: { id: string; is_pr: boolean }[] };

  await gotoHydrated(page, `/workouts/${w.id}`);
  const badge = page.getByRole('img', { name: 'Personal record' }).first();
  await expect(badge).toBeVisible();
  await expect(badge).toHaveScreenshot('pr-badge.png', {
    maxDiffPixelRatio: 0.02
  });
});
