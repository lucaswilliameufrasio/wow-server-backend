import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

vi.mock('$app/forms', () => ({
	applyAction: vi.fn(),
	enhance: () => ({ destroy: vi.fn() })
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));

afterEach(() => cleanup());

describe('register page', () => {
	it('Should render account creation fields and the sign-in link', () => {
		render(Page, { form: null });

		expect(document.title).toBe('Create an account | Tirion');
		expect(screen.getByRole('heading', { name: 'Create your account' })).toBeTruthy();
		expect(screen.getByLabelText('Account name')).toHaveAttribute('name', 'username');
		expect(screen.getByLabelText(/email/i)).toHaveAttribute('type', 'email');
		expect(screen.getByLabelText('Password')).toHaveAttribute('autocomplete', 'new-password');
		expect(screen.getByRole('button', { name: 'Create account' })).toBeTruthy();
		expect(screen.getByRole('link', { name: /back to sign in/i })).toHaveAttribute(
			'href',
			'/login'
		);
	});

	it('Should show the server validation message and preserve submitted fields', () => {
		render(Page, {
			form: {
				message: 'Account already exists',
				username: 'player',
				email: 'player@example.test'
			} as never
		});

		expect(screen.getByRole('alert')).toHaveTextContent('Account already exists');
		expect(screen.getByLabelText('Account name')).toHaveValue('player');
		expect(screen.getByLabelText(/email/i)).toHaveValue('player@example.test');
	});
});
