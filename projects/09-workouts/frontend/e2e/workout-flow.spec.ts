import { test, expect, type Page } from '@playwright/test';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq(prefix: string): string {
  return `${prefix} ${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

test('log a workout via the API and see PR badges on detail page', async ({
  page,
  request
}) => {
  // Seed an exercise.
  const exName = uniq('PR-Test Bench');
  const exRes = await request.post('http://localhost:3008/api/exercises', {
    data: { name: exName, muscle_group: 'chest' }
  });
  const exercise = (await exRes.json()) as { id: string };

  // First workout: a single set at 80kg x 5 = 400 kg·rep.
  await request.post('http://localhost:3008/api/workouts', {
    data: {
      name: uniq('Workout A'),
      sets: [{ exercise_id: exercise.id, weight_minor: 80_000, reps: 5, rir: 2 }]
    }
  });

  // Second workout: PR at 90kg x 5 = 450 kg·rep — should be flagged is_pr.
  const wRes = await request.post('http://localhost:3008/api/workouts', {
    data: {
      name: uniq('Workout B'),
      sets: [{ exercise_id: exercise.id, weight_minor: 90_000, reps: 5, rir: 1 }]
    }
  });
  const w = (await wRes.json()) as {
    id: string;
    sets: { id: string; is_pr: boolean }[];
  };
  expect(w.sets[0]?.is_pr).toBe(true);

  await gotoHydrated(page, `/workouts/${w.id}`);
  await expect(page.getByRole('img', { name: 'Personal record' })).toBeVisible();
});

test('unknown workout id returns the custom 404 page', async ({ page }) => {
  const res = await page.goto('/workouts/does-not-exist-99999');
  expect(res?.status()).toBe(404);
  await expect(page.getByRole('heading', { name: /not found/i })).toBeVisible();
});
