import { getAccessToken, requireAccount } from '$lib/server/auth';
import { listCharacters } from '$lib/server/characters';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, cookies }) => {
	const account = requireAccount(locals);
	const accessToken = getAccessToken(cookies);
	if (!accessToken) return { account, characters: [], error: 'Session expired.' };

	try {
		const response = await listCharacters(accessToken);
		return { account, characters: response.characters, error: null };
	} catch {
		return { account, characters: [], error: 'The roster could not be loaded right now.' };
	}
};
