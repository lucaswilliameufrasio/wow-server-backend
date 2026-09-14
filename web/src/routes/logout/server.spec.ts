import { beforeEach, describe, expect, it, vi } from 'vitest';
import { backendRequest } from '$lib/server/api';
import { clearAuthCookies, getAccessToken, getRefreshToken } from '$lib/server/auth';
import { POST } from './+server';

vi.mock('$lib/server/api', () => ({ backendRequest: vi.fn() }));
vi.mock('$lib/server/auth', () => ({
	clearAuthCookies: vi.fn(),
	getAccessToken: vi.fn(),
	getRefreshToken: vi.fn()
}));

const backendRequestMock = vi.mocked(backendRequest);
const clearAuthCookiesMock = vi.mocked(clearAuthCookies);
const getAccessTokenMock = vi.mocked(getAccessToken);
const getRefreshTokenMock = vi.mocked(getRefreshToken);

beforeEach(() => {
	backendRequestMock.mockReset();
	clearAuthCookiesMock.mockReset();
	getAccessTokenMock.mockReset();
	getRefreshTokenMock.mockReset();
});

describe('logout endpoint', () => {
	it('Should notify the backend, clear cookies, and redirect', async () => {
		getAccessTokenMock.mockReturnValue('access');
		getRefreshTokenMock.mockReturnValue('refresh');
		backendRequestMock.mockResolvedValue({});

		await expect(
			POST({ cookies: {}, url: new URL('https://localhost/logout') } as never)
		).rejects.toMatchObject({
			status: 303,
			location: '/login'
		});
		expect(backendRequestMock).toHaveBeenCalledWith('/v1/auth/logout', {
			method: 'POST',
			accessToken: 'access',
			body: JSON.stringify({ refresh_token: 'refresh' })
		});
		expect(clearAuthCookiesMock).toHaveBeenCalledWith({}, true);
	});

	it('Should clear cookies without calling the backend when unauthenticated', async () => {
		getAccessTokenMock.mockReturnValue(undefined);
		getRefreshTokenMock.mockReturnValue(undefined);

		await expect(
			POST({ cookies: {}, url: new URL('http://localhost/logout') } as never)
		).rejects.toMatchObject({
			status: 303,
			location: '/login'
		});
		expect(backendRequestMock).not.toHaveBeenCalled();
		expect(clearAuthCookiesMock).toHaveBeenCalledWith({}, false);
	});

	it('Should redirect even when backend logout fails', async () => {
		getAccessTokenMock.mockReturnValue('access');
		getRefreshTokenMock.mockReturnValue('refresh');
		backendRequestMock.mockRejectedValue(new Error('backend down'));

		await expect(
			POST({ cookies: {}, url: new URL('http://localhost/logout') } as never)
		).rejects.toMatchObject({
			status: 303,
			location: '/login'
		});
	});
});
