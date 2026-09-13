const races: Record<number, string> = {
	1: 'Human',
	2: 'Orc',
	3: 'Dwarf',
	4: 'Night Elf',
	5: 'Undead',
	6: 'Tauren',
	7: 'Gnome',
	8: 'Troll',
	10: 'Blood Elf',
	11: 'Draenei'
};

const classes: Record<number, string> = {
	1: 'Warrior',
	2: 'Paladin',
	3: 'Hunter',
	4: 'Rogue',
	5: 'Priest',
	6: 'Death Knight',
	7: 'Shaman',
	8: 'Mage',
	9: 'Warlock',
	11: 'Druid'
};

const factions: Record<number, 'Alliance' | 'Horde'> = {
	1: 'Alliance',
	3: 'Alliance',
	4: 'Alliance',
	7: 'Alliance',
	11: 'Alliance',
	2: 'Horde',
	5: 'Horde',
	6: 'Horde',
	8: 'Horde',
	10: 'Horde'
};

export function getRaceName(id: number) {
	return races[id] ?? `Race ${id}`;
}

export function getClassName(id: number) {
	return classes[id] ?? `Class ${id}`;
}

export function getFaction(race: number) {
	return factions[race] ?? 'Unknown faction';
}

export function formatMoney(copper: number) {
	const gold = Math.floor(copper / 10000);
	const silver = Math.floor((copper % 10000) / 100);
	const remainingCopper = copper % 100;
	return `${gold.toLocaleString()}g ${silver}s ${remainingCopper}c`;
}
