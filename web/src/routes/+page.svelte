<script lang="ts">
	import { resolve } from '$app/paths';
</script>

<svelte:head>
	<title>Tirion | Realm portal</title>
</svelte:head>

<main class="realm-grid min-h-screen">
	<div class="realm-frame flex min-h-screen flex-col justify-between">
		<header class="flex items-center justify-between border-b border-border/70 pb-5">
			<div class="flex items-center gap-3">
				<div class="size-3 rounded-full bg-emerald-300 shadow-[0_0_12px_currentColor]"></div>
				<p class="text-sm font-semibold tracking-[0.2em] text-foreground">TIRION</p>
			</div>
			<a
				class="text-sm text-muted-foreground transition hover:text-primary"
				href={resolve('/login')}>Sign in</a
			>
		</header>

		<section class="max-w-4xl py-20 sm:py-32">
			<p class="realm-kicker mb-6">Azeroth / player portal</p>
			<h1
				class="max-w-3xl text-5xl leading-[0.94] font-semibold tracking-[-0.05em] text-foreground sm:text-8xl"
			>
				Keep your place<br /><span class="text-primary">in the world.</span>
			</h1>
			<p class="mt-8 max-w-xl text-lg leading-8 text-muted-foreground">
				A focused account portal for your Tirion characters, their journeys and the realm they call
				home.
			</p>
			<a
				class="mt-10 inline-flex h-12 items-center bg-primary px-6 text-sm font-semibold text-primary-foreground transition hover:bg-primary/90"
				href={resolve('/login')}
			>
				Enter the realm <span class="ml-3">-&gt;</span>
			</a>
		</section>

		<footer
			class="flex flex-col gap-2 border-t border-border/70 py-5 text-xs text-muted-foreground sm:flex-row sm:items-center sm:justify-between"
		>
			<span>Realm status: <strong class="font-medium text-emerald-300">online</strong></span>
			<span>Player access only</span>
		</footer>
	</div>
</main>
