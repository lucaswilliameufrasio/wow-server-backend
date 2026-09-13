<script lang="ts">
	import { resolve } from '$app/paths';
</script>

<svelte:head>
	<title>Characters | Tirion</title>
</svelte:head>

<main class="realm-grid min-h-screen">
	<div class="realm-frame flex min-h-screen items-center justify-center">
		<section class="realm-panel max-w-lg p-8 text-center">
			<p class="realm-kicker mb-4">Roster / 01</p>
			<h1 class="text-3xl font-semibold text-foreground">The roster is being charted.</h1>
			<p class="mt-4 text-sm leading-6 text-muted-foreground">
				Character cards and live locations are the next layer of the command center.
			</p>
			<a
				class="mt-7 inline-flex text-sm font-semibold text-primary hover:underline"
				href={resolve('/dashboard')}>Return to command center</a
			>
		</section>
	</div>
</main>
