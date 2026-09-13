import { backendRequest, type CharacterListResponse, type CharacterLocationResponse } from './api';

export function listCharacters(accessToken: string) {
	return backendRequest<CharacterListResponse>('/v1/characters', { accessToken });
}

export function getCharacterLocation(accessToken: string, guid: string) {
	return backendRequest<CharacterLocationResponse>(`/v1/characters/${guid}/location`, {
		accessToken
	});
}
