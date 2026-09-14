import { expect, test } from '@playwright/test';

test('Should render the realm entry points', async ({ page }) => {
	const health = await page.request.get('/health');
	expect(health.ok()).toBe(true);

	await page.goto('/');
	await expect(page).toHaveTitle(/Tirion \| Realm portal/);
	await expect(page.getByRole('heading', { name: /keep your place/i })).toBeVisible();

	await page.goto('/login');
	await expect(page).toHaveTitle(/Enter the realm/);
	await expect(page.getByRole('button', { name: /continue to the realm/i })).toBeVisible();

	await page.goto('/register');
	await expect(page).toHaveTitle(/Create an account/);
	await expect(page.getByRole('button', { name: /create account/i })).toBeVisible();
});

test('Should protect the command center', async ({ page }) => {
	await page.goto('/dashboard');
	await expect(page).toHaveURL(/\/login$/);

	await page.goto('/characters/1');
	await expect(page).toHaveURL(/\/login$/);
});

test('Should log in and navigate to the command center', async ({ page, context }) => {
	await page.goto('/login');
	await page.locator('input[name=username]').fill('demo-player');
	await page.locator('input[name=password]').fill('DemoPassword9!');
	await page.getByRole('button', { name: /continue to the realm/i }).click();

	await expect(page).toHaveURL(/\/dashboard$/);
	await expect(page.getByRole('heading', { name: /welcome, demo-player/i })).toBeVisible();
	await expect(page.getByText('Demohero, level 80')).toBeVisible();

	const cookies = await context.cookies();
	expect(cookies.map((cookie) => cookie.name)).toEqual(
		expect.arrayContaining(['wow_access_token', 'wow_refresh_token'])
	);
});
