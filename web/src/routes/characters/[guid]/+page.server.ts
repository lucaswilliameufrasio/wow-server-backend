import { error } from '@sveltejs/kit';
import { getAccessToken, requireAccount } from '$lib/server/auth';
import { getCharacterLocation, listCharacters } from '$lib/server/characters';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, cookies, params }) => {
	const account = requireAccount(locals);
	const accessToken = getAccessToken(cookies);
	if (!accessToken) throw error(401, 'Session expired.');

	const response = await listCharacters(accessToken).catch(() => null);
	const character = response?.characters.find((item) => item.guid.toString() === params.guid);
	if (!character) throw error(404, 'Character not found.');

	const location = await getCharacterLocation(accessToken, params.guid).catch(() => null);
	return { account, character, location };
};
