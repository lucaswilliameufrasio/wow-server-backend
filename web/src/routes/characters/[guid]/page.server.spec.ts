import { describe, expect, it, vi } from 'vitest';
import { getAccessToken, requireAccount } from '$lib/server/auth';
import { getCharacterLocation, listCharacters } from '$lib/server/characters';
import { load } from './+page.server';

vi.mock('$lib/server/auth', () => ({
	getAccessToken: vi.fn(),
	requireAccount: vi.fn()
}));
vi.mock('$lib/server/characters', () => ({
	getCharacterLocation: vi.fn(),
	listCharacters: vi.fn()
}));

const account = { accountId: 1 };
const getAccessTokenMock = vi.mocked(getAccessToken);
const requireAccountMock = vi.mocked(requireAccount);
const getCharacterLocationMock = vi.mocked(getCharacterLocation);
const listCharactersMock = vi.mocked(listCharacters);

describe('character detail page server load', () => {
	it('Should reject an expired session', async () => {
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue(undefined);

		await expect(
			load({ locals: {}, cookies: {}, params: { guid: '42' } } as never)
		).rejects.toMatchObject({
			status: 401
		});
	});

	it('Should reject an unknown character', async () => {
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue('access');
		listCharactersMock.mockResolvedValue({ characters: [] } as never);

		await expect(
			load({ locals: {}, cookies: {}, params: { guid: '42' } } as never)
		).rejects.toMatchObject({
			status: 404
		});
	});

	it('Should return the character and location', async () => {
		const character = { guid: 42, name: 'Arthas' };
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue('access');
		listCharactersMock.mockResolvedValue({ characters: [character] } as never);
		getCharacterLocationMock.mockResolvedValue({ map: 'Icecrown' } as never);

		await expect(
			load({ locals: {}, cookies: {}, params: { guid: '42' } } as never)
		).resolves.toEqual({
			account,
			character,
			location: { map: 'Icecrown' }
		});
	});

	it('Should return the character when location lookup fails', async () => {
		const character = { guid: 42, name: 'Arthas' };
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue('access');
		listCharactersMock.mockResolvedValue({ characters: [character] } as never);
		getCharacterLocationMock.mockRejectedValue(new Error('not found'));

		await expect(
			load({ locals: {}, cookies: {}, params: { guid: '42' } } as never)
		).resolves.toMatchObject({
			account,
			character,
			location: null
		});
	});
});
