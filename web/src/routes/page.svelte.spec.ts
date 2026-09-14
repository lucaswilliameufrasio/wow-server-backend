import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));

describe('realm landing page', () => {
	it('Should present the realm entry points and online status', () => {
		render(Page);

		expect(document.title).toBe('Tirion | Realm portal');
		expect(screen.getByRole('heading', { name: /keep your place\s*in the world/i })).toBeTruthy();
		expect(screen.getByRole('link', { name: 'Sign in' })).toHaveAttribute('href', '/login');
		expect(screen.getByRole('link', { name: /enter the realm/i })).toHaveAttribute(
			'href',
			'/login'
		);
		expect(screen.getByText('online')).toBeTruthy();
	});
});
