<script lang="ts">
	import { enhance } from '$app/forms';
	import { resolve } from '$app/paths';
	import { ArrowRight, LockKeyhole, ShieldCheck, Sparkles } from '@lucide/svelte';
	import type { ActionData } from './$types';

	let { form }: { form: ActionData } = $props();
	let submitting = $state(false);
</script>

<svelte:head>
	<title>Enter the realm | Tirion</title>
</svelte:head>

<main class="realm-grid min-h-screen">
	<div class="realm-frame grid items-center lg:grid-cols-[1fr_0.82fr] lg:gap-16">
		<section class="order-2 py-8 lg:order-1 lg:py-16">
			<div class="mb-10 flex items-center gap-3">
				<div
					class="flex size-11 items-center justify-center border border-primary/60 bg-primary/10 text-primary"
				>
					<Sparkles size={21} strokeWidth={1.5} />
				</div>
				<div>
					<p class="text-sm font-semibold tracking-[0.2em] text-foreground">TIRION</p>
					<p class="text-xs tracking-[0.16em] text-muted-foreground uppercase">Realm operations</p>
				</div>
			</div>

			<p class="realm-kicker mb-5">Account portal / 01</p>
			<h1
				class="max-w-xl text-5xl leading-[0.95] font-semibold tracking-[-0.045em] text-foreground sm:text-7xl"
			>
				Your characters.<br />Your <span class="text-primary">realm.</span>
			</h1>
			<p class="mt-7 max-w-lg text-base leading-7 text-muted-foreground sm:text-lg">
				A quiet place to check your roster, follow your adventurers and stay connected to the world
				beyond the login screen.
			</p>

			<div class="mt-12 grid max-w-lg grid-cols-2 gap-3 sm:gap-4">
				<div class="realm-panel p-4 sm:p-5">
					<div class="mb-4 flex items-center gap-2 text-emerald-300">
						<span class="size-2 rounded-full bg-emerald-300 shadow-[0_0_12px_currentColor]"></span>
						<span class="text-xs font-semibold tracking-wider uppercase">Realm status</span>
					</div>
					<p class="text-lg font-medium text-foreground">Online</p>
					<p class="mt-1 text-xs text-muted-foreground">Tirion / Azeroth</p>
				</div>
				<div class="realm-panel p-4 sm:p-5">
					<div class="mb-4 flex items-center gap-2 text-primary">
						<ShieldCheck size={16} />
						<span class="text-xs font-semibold tracking-wider uppercase">Private access</span>
					</div>
					<p class="text-lg font-medium text-foreground">Protected</p>
					<p class="mt-1 text-xs text-muted-foreground">Session encryption enabled</p>
				</div>
			</div>
		</section>

		<section class="order-1 lg:order-2">
			<div class="realm-panel mx-auto max-w-md p-6 sm:p-8">
				<div class="mb-8">
					<div
						class="mb-5 flex size-12 items-center justify-center border border-primary/40 bg-primary/10 text-primary"
					>
						<LockKeyhole size={21} strokeWidth={1.7} />
					</div>
					<p class="realm-kicker mb-3">Welcome back</p>
					<h2 class="text-3xl font-semibold tracking-tight text-foreground">Enter the realm</h2>
					<p class="mt-2 text-sm leading-6 text-muted-foreground">
						Sign in to see your characters and their latest location.
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
						return async ({ update }) => {
							await update();
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
							class="h-12 w-full border border-border bg-background/70 px-4 text-sm text-foreground transition outline-none placeholder:text-muted-foreground/60 focus:border-primary focus:ring-2 focus:ring-primary/20"
							placeholder="Your account name"
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
							autocomplete="current-password"
							required
							class="h-12 w-full border border-border bg-background/70 px-4 text-sm text-foreground transition outline-none placeholder:text-muted-foreground/60 focus:border-primary focus:ring-2 focus:ring-primary/20"
							placeholder="Your password"
						/>
					</label>
					<button
						type="submit"
						disabled={submitting}
						class="group flex h-12 w-full items-center justify-center gap-2 bg-primary px-5 text-sm font-semibold text-primary-foreground transition hover:bg-primary/90 disabled:cursor-wait disabled:opacity-60"
					>
						{submitting ? 'Checking credentials...' : 'Continue to the realm'}
						{#if !submitting}<ArrowRight
								size={17}
								class="transition-transform group-hover:translate-x-1"
							/>{/if}
					</button>
				</form>

				<div class="mt-7 border-t border-border/70 pt-5 text-center text-sm text-muted-foreground">
					Need an account?
					<a
						class="font-medium text-primary underline-offset-4 hover:underline"
						href={resolve('/register')}>Create one</a
					>
				</div>
			</div>
		</section>
	</div>
</main>
