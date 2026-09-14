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
	map: 1,
	zone: 2,
	online: true,
	money: 123456
};

describe('characters page', () => {
	it('Should show the roster error', () => {
		render(Page, { data: { characters: [], error: 'Session expired.' } } as never);

		expect(document.title).toBe('Characters | Tirion');
		expect(screen.getByText('Session expired.')).toBeTruthy();
	});

	it('Should show the empty roster state', () => {
		render(Page, { data: { characters: [], error: null } } as never);

		expect(screen.getByRole('heading', { name: 'The roster is waiting.' })).toBeTruthy();
		expect(screen.getByText(/create a character in game/i)).toBeTruthy();
	});

	it('Should show character details and link to the detail page', () => {
		render(Page, { data: { characters: [character], error: null } } as never);

		expect(screen.getByRole('heading', { name: 'Your adventurers.' })).toBeTruthy();
		expect(screen.getByRole('heading', { name: 'Arthas' })).toBeTruthy();
		expect(screen.getByText('Online')).toBeTruthy();
		expect(screen.getByText(/Level 80/)).toBeTruthy();
		expect(screen.getByRole('link', { name: /Arthas/ })).toHaveAttribute('href', '/characters/7');
	});
});
