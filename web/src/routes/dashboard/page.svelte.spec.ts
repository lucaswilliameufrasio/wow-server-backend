import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));

afterEach(() => cleanup());

const character = {
	guid: 7,
	name: 'Arthas',
	race: 1,
	class_id: 2,
	gender: 0,
	level: 80,
	map: 0,
	zone: 0,
	online: true,
	money: 0
};

describe('dashboard page', () => {
	it('Should show account metrics and the online roster', () => {
		render(Page, {
			data: { account: { username: 'player' }, characters: [character], charactersError: null }
		} as never);

		expect(document.title).toBe('Command center | Tirion');
		expect(screen.getByRole('heading', { name: 'Welcome, player.' })).toBeTruthy();
		expect(screen.getByText(/Arthas,\s*level 80/)).toBeTruthy();
		expect(screen.getByText('Open roster')).toHaveAttribute('href', '/characters');
		expect(screen.getByText('Sign out')).toBeTruthy();
	});

	it('Should show the roster error', () => {
		render(Page, {
			data: { account: { username: 'player' }, characters: [], charactersError: 'Session expired.' }
		} as never);

		expect(screen.getByText('Session expired.')).toBeTruthy();
	});

	it('Should show the empty roster message', () => {
		render(Page, {
			data: { account: { username: 'player' }, characters: [], charactersError: null }
		} as never);

		expect(screen.getByText('No characters have crossed the gates yet.')).toBeTruthy();
	});
});
