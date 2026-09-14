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

describe('login page', () => {
	it('Should render the sign-in form and account link', () => {
		render(Page, { form: null });

		expect(document.title).toBe('Enter the realm | Tirion');
		expect(screen.getByRole('heading', { name: 'Enter the realm' })).toBeTruthy();
		expect(screen.getByLabelText('Account name')).toHaveAttribute('name', 'username');
		expect(screen.getByLabelText('Password')).toHaveAttribute('type', 'password');
		expect(screen.getByRole('button', { name: /continue to the realm/i })).toBeTruthy();
		expect(screen.getByRole('link', { name: 'Create one' })).toHaveAttribute('href', '/register');
	});

	it('Should show the server validation message and preserve the username', () => {
		render(Page, { form: { message: 'Invalid credentials', username: 'player' } as never });

		expect(screen.getByRole('alert')).toHaveTextContent('Invalid credentials');
		expect(screen.getByLabelText('Account name')).toHaveValue('player');
	});
});
