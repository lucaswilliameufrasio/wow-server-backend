import { redirect } from '@sveltejs/kit';
import { dev } from '$app/environment';
import type { Cookies } from '@sveltejs/kit';
import {
	backendRequest,
	BackendError,
	type AuthMeResponse,
	type RefreshResponse,
	type SignInResponse
} from './api';

const accessCookie = 'wow_access_token';
const refreshCookie = 'wow_refresh_token';

const cookieOptions = {
	path: '/',
	httpOnly: true,
	sameSite: 'lax' as const
};

function getCookieOptions(secure = !dev) {
	return { ...cookieOptions, secure };
}

function setTokenCookies(
	cookies: Cookies,
	tokens: Pick<
		SignInResponse,
		'access_token' | 'refresh_token' | 'expires_in_seconds' | 'refresh_expires_in_seconds'
	>,
	secure = !dev
) {
	cookies.set(accessCookie, tokens.access_token, {
		...getCookieOptions(secure),
		maxAge: tokens.expires_in_seconds
	});
	cookies.set(refreshCookie, tokens.refresh_token, {
		...getCookieOptions(secure),
		maxAge: tokens.refresh_expires_in_seconds
	});
}

export function clearAuthCookies(cookies: Cookies, secure = !dev) {
	cookies.delete(accessCookie, getCookieOptions(secure));
	cookies.delete(refreshCookie, getCookieOptions(secure));
}

export async function signIn(cookies: Cookies, username: string, password: string, secure = !dev) {
	const response = await backendRequest<SignInResponse>('/v1/auth/sign-in', {
		method: 'POST',
		body: JSON.stringify({ username, password })
	});
	setTokenCookies(cookies, response, secure);
	return response;
}

export async function getCurrentAccount(
	cookies: Cookies,
	secure = !dev
): Promise<AuthMeResponse | null> {
	const accessToken = cookies.get(accessCookie);
	if (!accessToken) return null;

	try {
		return await backendRequest<AuthMeResponse>('/v1/auth/me', { accessToken });
	} catch (error) {
		if (!(error instanceof BackendError) || error.status !== 401) {
			return null;
		}
		const refreshToken = cookies.get(refreshCookie);
		if (!refreshToken) {
			clearAuthCookies(cookies);
			return null;
		}

		try {
			const refreshed = await backendRequest<RefreshResponse>('/v1/auth/refresh', {
				method: 'POST',
				body: JSON.stringify({ refresh_token: refreshToken })
			});
			setTokenCookies(cookies, refreshed, secure);
			return await backendRequest<AuthMeResponse>('/v1/auth/me', {
				accessToken: refreshed.access_token
			});
		} catch {
			clearAuthCookies(cookies);
			return null;
		}
	}
}

export function requireAccount(locals: App.Locals) {
	if (!locals.session) throw redirect(303, '/login');
	return locals.session;
}

export function getAccessToken(cookies: Cookies) {
	return cookies.get(accessCookie);
}

export function getRefreshToken(cookies: Cookies) {
	return cookies.get(refreshCookie);
}
