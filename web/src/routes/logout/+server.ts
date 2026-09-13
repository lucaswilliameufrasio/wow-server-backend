import { redirect } from '@sveltejs/kit';
import { backendRequest } from '$lib/server/api';
import { clearAuthCookies, getAccessToken, getRefreshToken } from '$lib/server/auth';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ cookies }) => {
	const accessToken = getAccessToken(cookies);
	const refreshToken = getRefreshToken(cookies);

	if (accessToken) {
		await backendRequest('/v1/auth/logout', {
			method: 'POST',
			accessToken,
			body: JSON.stringify({ refresh_token: refreshToken })
		}).catch(() => undefined);
	}

	clearAuthCookies(cookies);
	throw redirect(303, '/login');
};
