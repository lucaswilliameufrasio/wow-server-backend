import { fail, redirect } from '@sveltejs/kit';
import { BackendError, backendRequest } from '$lib/server/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = ({ locals }) => {
	if (locals.session) throw redirect(303, '/dashboard');
};

export const actions: Actions = {
	default: async ({ request }) => {
		const form = await request.formData();
		const username = String(form.get('username') ?? '').trim();
		const email = String(form.get('email') ?? '').trim();
		const password = String(form.get('password') ?? '');

		if (!username || !password) {
			return fail(400, { username, email, message: 'Account name and password are required.' });
		}

		try {
			await backendRequest('/v1/auth/register', {
				method: 'POST',
				body: JSON.stringify({ username, password, email: email || undefined })
			});
		} catch (error) {
			if (error instanceof BackendError) {
				return fail(error.status === 409 ? 409 : 400, { username, email, message: error.message });
			}
			return fail(502, {
				username,
				email,
				message: 'The realm gateway is temporarily unavailable.'
			});
		}

		throw redirect(303, '/login?created=1');
	}
};
