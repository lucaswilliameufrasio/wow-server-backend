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

describe('characters page server load', () => {
	it('Should return a session error without an access token', async () => {
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue(undefined);

		await expect(load({ locals: {}, cookies: {} } as never)).resolves.toEqual({
			account,
			characters: [],
			error: 'Session expired.'
		});
	});

	it('Should return characters from the roster endpoint', async () => {
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue('access');
		listCharactersMock.mockResolvedValue({ characters: [{ guid: 7 }] } as never);

		await expect(load({ locals: {}, cookies: {} } as never)).resolves.toEqual({
			account,
			characters: [{ guid: 7 }],
			error: null
		});
	});

	it('Should return an error when the roster endpoint fails', async () => {
		requireAccountMock.mockReturnValue(account as never);
		getAccessTokenMock.mockReturnValue('access');
		listCharactersMock.mockRejectedValue(new Error('backend down'));

		await expect(load({ locals: {}, cookies: {} } as never)).resolves.toMatchObject({
			account,
			characters: [],
			error: expect.stringContaining('roster')
		});
	});
});
