import { describe, expect, it, vi } from 'vitest';
import { BackendError, backendRequest } from '$lib/server/api';
import { actions, load } from './+page.server';

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

const backendRequestMock = vi.mocked(backendRequest);

function requestWith(values: Record<string, string>) {
	return {
		formData: async () => {
			const form = new FormData();
			for (const [name, value] of Object.entries(values)) form.append(name, value);
			return form;
		}
	};
}

describe('register page server actions', () => {
	it('Should reject missing credentials', async () => {
		const result = await actions.default({ request: requestWith({}) } as never);

		expect(result).toMatchObject({ status: 400, data: { message: expect.any(String) } });
	});

	it('Should redirect to login after registration', async () => {
		backendRequestMock.mockResolvedValue({});

		await expect(
			actions.default({
				request: requestWith({
					username: 'player',
					email: 'player@example.test',
					password: 'secret'
				})
			} as never)
		).rejects.toMatchObject({ status: 303, location: '/login?created=1' });
		expect(backendRequestMock).toHaveBeenCalledWith('/v1/auth/register', {
			method: 'POST',
			body: JSON.stringify({ username: 'player', password: 'secret', email: 'player@example.test' })
		});
	});

	it('Should return a conflict for an existing account', async () => {
		backendRequestMock.mockRejectedValue(new BackendError(409, 'Already exists'));

		const result = await actions.default({
			request: requestWith({ username: 'player', password: 'secret' })
		} as never);

		expect(result).toMatchObject({ status: 409, data: { message: 'Already exists' } });
	});

	it('Should return a gateway error for unexpected failures', async () => {
		backendRequestMock.mockRejectedValue(new Error('network down'));

		const result = await actions.default({
			request: requestWith({ username: 'player', password: 'secret' })
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
