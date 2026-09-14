import { describe, expect, it, vi } from 'vitest';
import { BackendError } from '$lib/server/api';
import { signIn } from '$lib/server/auth';
import { actions, load } from './+page.server';

vi.mock('$lib/server/auth', () => ({ signIn: vi.fn() }));
vi.mock('$lib/server/api', () => ({
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

const signInMock = vi.mocked(signIn);

function requestWith(values: Record<string, string>) {
	return {
		formData: async () => {
			const form = new FormData();
			for (const [name, value] of Object.entries(values)) form.append(name, value);
			return form;
		}
	};
}

describe('login page server actions', () => {
	it('Should reject missing credentials', async () => {
		const result = await actions.default({
			request: requestWith({}),
			cookies: {},
			url: new URL('http://localhost/login')
		} as never);

		expect(result).toMatchObject({ status: 400, data: { message: expect.any(String) } });
	});

	it('Should redirect to the dashboard after sign in', async () => {
		signInMock.mockResolvedValue({} as never);

		await expect(
			actions.default({
				request: requestWith({ username: 'player', password: 'secret' }),
				cookies: {},
				url: new URL('http://localhost/login')
			} as never)
		).rejects.toMatchObject({ status: 303, location: '/dashboard' });
		expect(signInMock).toHaveBeenCalledWith({}, 'player', 'secret', false);
	});

	it('Should return the backend validation error', async () => {
		signInMock.mockRejectedValue(new BackendError(401, 'Invalid credentials'));

		const result = await actions.default({
			request: requestWith({ username: 'player', password: 'secret' }),
			cookies: {},
			url: new URL('http://localhost/login')
		} as never);

		expect(result).toMatchObject({ status: 401, data: { message: 'Invalid credentials' } });
	});

	it('Should return a gateway error for unexpected failures', async () => {
		signInMock.mockRejectedValue(new Error('network down'));

		const result = await actions.default({
			request: requestWith({ username: 'player', password: 'secret' }),
			cookies: {},
			url: new URL('http://localhost/login')
		} as never);

		expect(result).toMatchObject({
			status: 502,
			data: { message: expect.stringContaining('gateway') }
		});
	});

	it('Should redirect an existing session to the dashboard', async () => {
		try {
			load({ locals: { session: { accountId: 1 } } } as never);
			throw new Error('Expected redirect');
		} catch (error) {
			expect(error).toMatchObject({ status: 303, location: '/dashboard' });
		}
	});
});
