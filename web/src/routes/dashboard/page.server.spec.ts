import { describe, expect, it, vi } from 'vitest';
import { getAccessToken, requireAccount } from '$lib/server/auth';
import { listCharacters } from '$lib/server/characters';
import { load } from './+page.server';

vi.mock('$lib/server/auth', () => ({
	getAccessToken: vi.fn(),
	requireAccount: vi.fn()
}));
vi.mock('$lib/server/characters', () => ({ listCharacters: vi.fn() }));

const account = { accountId: 1 };
const getAccessTokenMock = vi.mocked(getAccessToken);
const requireAccountMock = vi.mocked(requireAccount);
const listCharactersMock = vi.mocked(listCharacters);

describe('dashboard page server load', () => {
	it('Should return an empty roster when the session token is missing', async () => {
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue(undefined);

		await expect(load({ locals: {}, cookies: {} } as never)).resolves.toEqual({
			account,
			characters: [],
			charactersError: 'Session expired.'
		});
	});

	it('Should return the loaded roster', async () => {
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue('access');
		listCharactersMock.mockResolvedValue({ characters: [{ guid: 7 }] } as never);

		await expect(load({ locals: {}, cookies: {} } as never)).resolves.toEqual({
			account,
			characters: [{ guid: 7 }],
			charactersError: null
		});
		expect(listCharactersMock).toHaveBeenCalledWith('access');
	});

	it('Should return a friendly error when the roster request fails', async () => {
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue('access');
		listCharactersMock.mockRejectedValue(new Error('backend down'));

		await expect(load({ locals: {}, cookies: {} } as never)).resolves.toMatchObject({
			account,
			characters: [],
			charactersError: expect.stringContaining('roster')
		});
	});
});
