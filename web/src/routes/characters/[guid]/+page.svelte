<script lang="ts">
	import { ArrowLeft, Compass, MapPin, Navigation, Shield } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { formatMoney, getClassName, getFaction, getRaceName } from '$lib/game-data';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	let character = $derived(data.character);
</script>

<svelte:head>
	<title>{character.name} | Tirion</title>
</svelte:head>

<main class="realm-grid min-h-screen">
	<div class="realm-frame">
		<header class="mb-12 flex items-center justify-between border-b border-border/70 pb-5">
			<a
				class="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
				href={resolve('/characters')}><ArrowLeft size={16} /> Character registry</a
			>
			<span
				class="flex items-center gap-2 text-xs {character.online
					? 'text-emerald-300'
					: 'text-muted-foreground'}"
				><span class="size-2 rounded-full bg-current"></span>{character.online
					? 'Online'
					: 'Offline'}</span
			>
		</header>

		<section class="mb-10 max-w-3xl">
			<p class="realm-kicker mb-4">Character dossier / {character.guid}</p>
			<h1 class="text-5xl font-semibold tracking-tight text-foreground sm:text-7xl">
				{character.name}
			</h1>
			<p class="mt-4 text-lg text-muted-foreground">
				Level {character.level}
				{getRaceName(character.race)}
				{getClassName(character.class_id)} of the {getFaction(character.race)}.
			</p>
		</section>

		<div class="grid gap-4 md:grid-cols-[1.2fr_0.8fr]">
			<section class="realm-panel realm-grid p-6 sm:p-8">
				<div
					class="mb-12 flex size-14 items-center justify-center border border-primary/40 bg-primary/10 text-primary"
				>
					<Navigation size={25} />
				</div>
				<p class="realm-kicker mb-3">Last known location</p>
				{#if data.location}
					<h2 class="text-3xl font-semibold text-foreground">Map {data.location.map}</h2>
					<p class="mt-2 text-muted-foreground">Zone {data.location.zone}</p>
					<div class="mt-8 grid grid-cols-3 gap-3 border-t border-border/70 pt-5 text-sm">
						<div>
							<p class="text-xs text-muted-foreground">X</p>
							<p class="mt-1 text-foreground">{data.location.position_x.toFixed(2)}</p>
						</div>
						<div>
							<p class="text-xs text-muted-foreground">Y</p>
							<p class="mt-1 text-foreground">{data.location.position_y.toFixed(2)}</p>
						</div>
						<div>
							<p class="text-xs text-muted-foreground">Z</p>
							<p class="mt-1 text-foreground">{data.location.position_z.toFixed(2)}</p>
						</div>
					</div>
				{:else}
					<h2 class="text-3xl font-semibold text-foreground">Location unavailable</h2>
					<p class="mt-2 text-muted-foreground">
						The realm did not return a current position for this character.
					</p>
				{/if}
			</section>

			<section class="space-y-4">
				<div class="realm-panel p-6">
					<div class="mb-7 flex items-center gap-3 text-primary">
						<Shield size={18} />
						<p class="realm-kicker">Identity</p>
					</div>
					<p class="text-sm text-muted-foreground">Race</p>
					<p class="mt-1 text-lg text-foreground">{getRaceName(character.race)}</p>
					<p class="mt-5 text-sm text-muted-foreground">Class</p>
					<p class="mt-1 text-lg text-foreground">{getClassName(character.class_id)}</p>
				</div>
				<div class="realm-panel p-6">
					<div class="mb-7 flex items-center gap-3 text-primary">
						<Compass size={18} />
						<p class="realm-kicker">Purse</p>
					</div>
					<p class="text-2xl text-primary">{formatMoney(character.money)}</p>
					<p class="mt-2 text-xs text-muted-foreground">Current carried currency</p>
				</div>
				<div class="realm-panel p-6">
					<div class="mb-7 flex items-center gap-3 text-primary">
						<MapPin size={18} />
						<p class="realm-kicker">Coordinates</p>
					</div>
					<p class="text-sm text-muted-foreground">Map {character.map} / Zone {character.zone}</p>
				</div>
			</section>
		</div>
	</div>
</main>
