import { describe, expect, it, vi } from 'vitest';
import { backendRequest } from './api';
import { getCharacterLocation, listCharacters } from './characters';

vi.mock('./api', () => ({ backendRequest: vi.fn() }));

const backendRequestMock = vi.mocked(backendRequest);

describe('character API helpers', () => {
	it('Should list characters with the access token', async () => {
		backendRequestMock.mockResolvedValue({ characters: [] });

		await listCharacters('access');

		expect(backendRequestMock).toHaveBeenCalledWith('/v1/characters', { accessToken: 'access' });
	});

	it('Should request a character location with the access token', async () => {
		backendRequestMock.mockResolvedValue({ map: 'Elwynn Forest' });

		await getCharacterLocation('access', '42');

		expect(backendRequestMock).toHaveBeenCalledWith('/v1/characters/42/location', {
			accessToken: 'access'
		});
	});
});
