<script lang="ts">
	import { ArrowRight, LogOut, Users } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Command center | Tirion</title>
</svelte:head>

<main class="realm-grid min-h-screen">
	<div class="realm-frame">
		<header class="mb-12 flex items-center justify-between border-b border-border/70 pb-5">
			<div class="flex items-center gap-3">
				<div class="size-3 rounded-full bg-emerald-300 shadow-[0_0_12px_currentColor]"></div>
				<div>
					<p class="text-sm font-semibold tracking-[0.2em] text-foreground">TIRION</p>
					<p class="text-[0.65rem] tracking-[0.16em] text-muted-foreground uppercase">
						Command center
					</p>
				</div>
			</div>
			<form method="POST" action="/logout">
				<button
					class="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
					type="submit"
				>
					<LogOut size={16} /> Sign out
				</button>
			</form>
		</header>

		<div class="mb-12 max-w-2xl">
			<p class="realm-kicker mb-4">Account overview</p>
			<h1 class="text-4xl font-semibold tracking-tight text-foreground sm:text-6xl">
				Welcome, {data.account.username}.
			</h1>
			<p class="mt-4 text-lg leading-7 text-muted-foreground">
				Your character roster is ready when you are.
			</p>
		</div>

		<div class="grid gap-4 sm:grid-cols-2">
			<div class="realm-panel p-6">
				<div
					class="mb-10 flex size-11 items-center justify-center border border-primary/40 bg-primary/10 text-primary"
				>
					<Users size={20} />
				</div>
				<p class="realm-kicker mb-3">Characters</p>
				<h2 class="text-2xl font-semibold text-foreground">Your roster</h2>
				<p class="mt-2 max-w-sm text-sm leading-6 text-muted-foreground">
					See your heroes, levels and where they last stood in Azeroth.
				</p>
				<a
					class="mt-8 inline-flex items-center gap-2 text-sm font-semibold text-primary hover:underline"
					href={resolve('/characters')}
				>
					Open roster <ArrowRight size={16} />
				</a>
			</div>
			<div class="realm-panel p-6">
				<div class="mb-10 flex items-center gap-2 text-emerald-300">
					<span class="size-2 rounded-full bg-current shadow-[0_0_12px_currentColor]"></span><span
						class="text-xs font-semibold tracking-[0.16em] uppercase">Online</span
					>
				</div>
				<p class="realm-kicker mb-3">Realm status</p>
				<h2 class="text-2xl font-semibold text-foreground">Tirion is online</h2>
				<p class="mt-2 max-w-sm text-sm leading-6 text-muted-foreground">
					The gates are open. More live realm information will appear here soon.
				</p>
			</div>
		</div>
	</div>
</main>
