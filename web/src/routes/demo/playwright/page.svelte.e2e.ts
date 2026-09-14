import { expect, test } from '@playwright/test';

test('Should show the expected heading', async ({ page }) => {
	await page.goto('/demo/playwright');
	await expect(page.locator('h1')).toBeVisible();
});
