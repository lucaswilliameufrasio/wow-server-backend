<script lang="ts">
	import { applyAction, enhance } from '$app/forms';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import type { Pathname } from '$app/types';
	import { ArrowLeft, UserRoundPlus } from '@lucide/svelte';
	import type { ActionData } from './$types';

	let { form }: { form: ActionData } = $props();
	let submitting = $state(false);
</script>

<svelte:head>
	<title>Create an account | Tirion</title>
</svelte:head>

<main class="realm-grid min-h-screen">
	<div class="realm-frame flex min-h-screen items-center justify-center">
		<section class="realm-panel w-full max-w-md p-6 sm:p-8">
			<a
				class="mb-10 inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
				href={resolve('/login')}
			>
				<ArrowLeft size={16} /> Back to sign in
			</a>
			<div class="mb-8">
				<div
					class="mb-5 flex size-12 items-center justify-center border border-primary/40 bg-primary/10 text-primary"
				>
					<UserRoundPlus size={21} strokeWidth={1.7} />
				</div>
				<p class="realm-kicker mb-3">New adventurer</p>
				<h1 class="text-3xl font-semibold tracking-tight text-foreground">Create your account</h1>
				<p class="mt-2 text-sm leading-6 text-muted-foreground">
					Choose your account details and prepare to enter the realm.
				</p>
			</div>

			{#if form?.message}
				<div
					class="mb-5 border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-red-200"
					role="alert"
				>
					{form.message}
				</div>
			{/if}

			<form
				method="POST"
				use:enhance={() => {
					submitting = true;
					return async ({ result }) => {
						if (result.type === 'redirect') {
							await goto(resolve(result.location as Pathname));
							return;
						}
						await applyAction(result);
						submitting = false;
					};
				}}
				class="space-y-5"
			>
				<label class="block">
					<span
						class="mb-2 block text-xs font-semibold tracking-[0.14em] text-muted-foreground uppercase"
						>Account name</span
					>
					<input
						name="username"
						autocomplete="username"
						required
						value={form?.username ?? ''}
						class="h-12 w-full border border-border bg-background/70 px-4 text-sm text-foreground transition outline-none focus:border-primary focus:ring-2 focus:ring-primary/20"
					/>
				</label>
				<label class="block">
					<span
						class="mb-2 block text-xs font-semibold tracking-[0.14em] text-muted-foreground uppercase"
						>Email <span class="tracking-normal normal-case">(optional)</span></span
					>
					<input
						name="email"
						type="email"
						autocomplete="email"
						value={form?.email ?? ''}
						class="h-12 w-full border border-border bg-background/70 px-4 text-sm text-foreground transition outline-none focus:border-primary focus:ring-2 focus:ring-primary/20"
					/>
				</label>
				<label class="block">
					<span
						class="mb-2 block text-xs font-semibold tracking-[0.14em] text-muted-foreground uppercase"
						>Password</span
					>
					<input
						name="password"
						type="password"
						autocomplete="new-password"
						required
						class="h-12 w-full border border-border bg-background/70 px-4 text-sm text-foreground transition outline-none focus:border-primary focus:ring-2 focus:ring-primary/20"
					/>
				</label>
				<button
					type="submit"
					disabled={submitting}
					class="flex h-12 w-full items-center justify-center bg-primary px-5 text-sm font-semibold text-primary-foreground transition hover:bg-primary/90 disabled:cursor-wait disabled:opacity-60"
				>
					{submitting ? 'Creating account...' : 'Create account'}
				</button>
			</form>
		</section>
	</div>
</main>
