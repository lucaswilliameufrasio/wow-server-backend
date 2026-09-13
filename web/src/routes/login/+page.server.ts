import { fail, redirect } from '@sveltejs/kit';
import { BackendError } from '$lib/server/api';
import { signIn } from '$lib/server/auth';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = ({ locals }) => {
	if (locals.session) throw redirect(303, '/dashboard');
};

export const actions: Actions = {
	default: async ({ request, cookies, url }) => {
		const form = await request.formData();
		const username = String(form.get('username') ?? '').trim();
		const password = String(form.get('password') ?? '');

		if (!username || !password) {
			return fail(400, { username, message: 'Enter your account name and password.' });
		}

		try {
			await signIn(cookies, username, password, url.protocol === 'https:');
		} catch (error) {
			if (error instanceof BackendError) {
				return fail(error.status === 401 ? 401 : 400, {
					username,
					message: error.message
				});
			}
			return fail(502, { username, message: 'The realm gateway is temporarily unavailable.' });
		}

		throw redirect(303, '/dashboard');
	}
};
