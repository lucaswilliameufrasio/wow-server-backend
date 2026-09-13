<script lang="ts">
	import { ArrowLeft, MapPin, Swords } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { formatMoney, getClassName, getFaction, getRaceName } from '$lib/game-data';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Characters | Tirion</title>
</svelte:head>

<main class="realm-grid min-h-screen">
	<div class="realm-frame">
		<header class="mb-12 flex items-center justify-between border-b border-border/70 pb-5">
			<a
				class="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
				href={resolve('/dashboard')}
			>
				<ArrowLeft size={16} /> Command center
			</a>
			<p class="realm-kicker">Roster / {data.characters.length.toString().padStart(2, '0')}</p>
		</header>

		<div class="mb-10 max-w-2xl">
			<p class="realm-kicker mb-4">Character registry</p>
			<h1 class="text-4xl font-semibold tracking-tight text-foreground sm:text-6xl">
				Your adventurers.
			</h1>
			<p class="mt-4 text-lg leading-7 text-muted-foreground">
				A live view of every character attached to your account.
			</p>
		</div>

		{#if data.error}
			<div class="realm-panel border-amber-300/30 p-6 text-amber-100">{data.error}</div>
		{:else if data.characters.length === 0}
			<div class="realm-panel p-8">
				<p class="realm-kicker mb-3">No characters found</p>
				<h2 class="text-2xl font-semibold text-foreground">The roster is waiting.</h2>
				<p class="mt-2 text-sm text-muted-foreground">
					Create a character in game and return here to see their journey.
				</p>
			</div>
		{:else}
			<div class="grid gap-4 md:grid-cols-2">
				{#each data.characters as character (character.guid)}
					<a
						class="realm-panel group block p-5 transition hover:-translate-y-0.5 hover:border-primary/70"
						href={resolve(`/characters/${character.guid}`)}
					>
						<div class="mb-7 flex items-start justify-between gap-4">
							<div class="flex items-center gap-3">
								<div
									class="flex size-11 items-center justify-center border border-primary/40 bg-primary/10 text-primary"
								>
									<Swords size={19} />
								</div>
								<div>
									<h2 class="text-xl font-semibold text-foreground">{character.name}</h2>
									<p class="text-xs text-muted-foreground">
										Level {character.level}
										{getClassName(character.class_id)}
									</p>
								</div>
							</div>
							<span
								class="flex items-center gap-2 text-xs {character.online
									? 'text-emerald-300'
									: 'text-muted-foreground'}"
								><span class="size-2 rounded-full bg-current"></span>{character.online
									? 'Online'
									: 'Offline'}</span
							>
						</div>
						<div class="grid grid-cols-2 gap-3 border-t border-border/70 pt-4 text-xs">
							<div>
								<p class="text-muted-foreground">Origin</p>
								<p class="mt-1 text-foreground">
									{getRaceName(character.race)} / {getFaction(character.race)}
								</p>
							</div>
							<div>
								<p class="text-muted-foreground">Purse</p>
								<p class="mt-1 text-primary">{formatMoney(character.money)}</p>
							</div>
						</div>
						<div class="mt-4 flex items-center gap-2 text-xs text-muted-foreground">
							<MapPin size={14} /> Map {character.map} / Zone {character.zone}
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</main>
