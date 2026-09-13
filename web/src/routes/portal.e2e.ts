import { expect, test } from '@playwright/test';

test('renders the realm entry points', async ({ page }) => {
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

test('protects the command center', async ({ page }) => {
	await page.goto('/dashboard');
	await expect(page).toHaveURL(/\/login$/);
});
