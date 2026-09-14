import { afterEach, describe, expect, it, vi } from 'vitest';
import { BackendError, backendRequest } from './api';
import {
	clearAuthCookies,
	getAccessToken,
	getCurrentAccount,
	getRefreshToken,
	requireAccount,
	signIn
} from './auth';

vi.mock('./api', () => ({
	BackendError: class BackendError extends Error {
		constructor(
			public readonly status: number,
			message: string
		) {
			super(message);
		}
	},
	backendRequest: vi.fn()
}));

const backendRequestMock = vi.mocked(backendRequest);

function createCookies(initial: Record<string, string> = {}) {
	const values = new Map(Object.entries(initial));
	return {
		get: vi.fn((name: string) => values.get(name)),
		set: vi.fn((name: string, value: string) => values.set(name, value)),
		delete: vi.fn((name: string) => values.delete(name))
	};
}

afterEach(() => {
	backendRequestMock.mockReset();
});

describe('auth cookies', () => {
	it('Should sign in and set access and refresh cookies', async () => {
		const cookies = createCookies();
		backendRequestMock.mockResolvedValue({
			access_token: 'access',
			refresh_token: 'refresh',
			expires_in_seconds: 60,
			refresh_expires_in_seconds: 3600
		});

		await signIn(cookies as never, 'player', 'password', false);

		expect(cookies.set).toHaveBeenCalledWith(
			'wow_access_token',
			'access',
			expect.objectContaining({ secure: false, httpOnly: true, maxAge: 60 })
		);
		expect(cookies.set).toHaveBeenCalledWith(
			'wow_refresh_token',
			'refresh',
			expect.objectContaining({ secure: false, httpOnly: true, maxAge: 3600 })
		);
	});

	it('Should return no account when the access cookie is absent', async () => {
		const cookies = createCookies();

		await expect(getCurrentAccount(cookies as never, false)).resolves.toBeNull();
		expect(backendRequestMock).not.toHaveBeenCalled();
	});

	it('Should return no account for a non-authentication backend error', async () => {
		const cookies = createCookies({ wow_access_token: 'access' });
		backendRequestMock.mockRejectedValue(new BackendError(500, 'server error'));

		await expect(getCurrentAccount(cookies as never, false)).resolves.toBeNull();
		expect(backendRequestMock).toHaveBeenCalledTimes(1);
	});

	it('Should clear cookies when the access token expires without a refresh token', async () => {
		const cookies = createCookies({ wow_access_token: 'expired' });
		backendRequestMock.mockRejectedValue(new BackendError(401, 'expired'));

		await expect(getCurrentAccount(cookies as never, false)).resolves.toBeNull();
		expect(cookies.delete).toHaveBeenCalledTimes(2);
	});

	it('Should load the current account with a valid access token', async () => {
		const cookies = createCookies({ wow_access_token: 'access' });
		backendRequestMock.mockResolvedValue({ account_id: 1, username: 'player' });

		await expect(getCurrentAccount(cookies as never, false)).resolves.toMatchObject({
			account_id: 1,
			username: 'player'
		});
		expect(backendRequestMock).toHaveBeenCalledWith('/v1/auth/me', { accessToken: 'access' });
	});

	it('Should refresh tokens after an expired access token', async () => {
		const cookies = createCookies({
			wow_access_token: 'expired',
			wow_refresh_token: 'refresh'
		});
		backendRequestMock
			.mockRejectedValueOnce(new BackendError(401, 'expired'))
			.mockResolvedValueOnce({
				access_token: 'new-access',
				refresh_token: 'new-refresh',
				expires_in_seconds: 60,
				refresh_expires_in_seconds: 3600
			})
			.mockResolvedValueOnce({ account_id: 1, username: 'player' });

		await expect(getCurrentAccount(cookies as never, false)).resolves.toMatchObject({
			username: 'player'
		});
		expect(cookies.set).toHaveBeenCalledWith(
			'wow_access_token',
			'new-access',
			expect.objectContaining({ secure: false })
		);
	});

	it('Should clear cookies when refresh fails', async () => {
		const cookies = createCookies({ wow_access_token: 'expired', wow_refresh_token: 'refresh' });
		backendRequestMock
			.mockRejectedValueOnce(new BackendError(401, 'expired'))
			.mockRejectedValueOnce(new BackendError(401, 'revoked'));

		await expect(getCurrentAccount(cookies as never, false)).resolves.toBeNull();
		expect(cookies.delete).toHaveBeenCalledWith(
			'wow_access_token',
			expect.objectContaining({ secure: false })
		);
		expect(cookies.delete).toHaveBeenCalledWith(
			'wow_refresh_token',
			expect.objectContaining({ secure: false })
		);
	});

	it('Should clear both cookies explicitly', () => {
		const cookies = createCookies({ wow_access_token: 'access', wow_refresh_token: 'refresh' });

		clearAuthCookies(cookies as never, false);

		expect(cookies.delete).toHaveBeenCalledTimes(2);
	});

	it('Should return the session account when authentication is present', () => {
		const account = { accountId: 1 } as never;

		expect(requireAccount({ session: account })).toBe(account);
	});

	it('Should redirect when authentication is missing', () => {
		expect(() => requireAccount({ session: null })).toThrow();
	});

	it('Should read access and refresh token cookies', () => {
		const cookies = createCookies({ wow_access_token: 'access', wow_refresh_token: 'refresh' });

		expect(getAccessToken(cookies as never)).toBe('access');
		expect(getRefreshToken(cookies as never)).toBe('refresh');
	});
});
