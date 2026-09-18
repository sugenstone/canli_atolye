<script lang="ts">
	/** Giriş sayfası — Faz 1. Başarılı girişte dashboard'a yönlenir. */
	import { goto } from '$app/navigation';
	import { Card, Input, Label } from 'flowbite-svelte';
	import AppButton from '$lib/components/ui/AppButton.svelte';
	import { authService } from '$lib/services/api/auth';
	import { ApiError } from '$lib/services/api/client';
	import { session } from '$lib/stores/session.svelte';

	let email = $state('');
	let password = $state('');
	let error = $state<string | null>(null);
	let submitting = $state(false);

	async function onSubmit(e: SubmitEvent) {
		e.preventDefault();
		if (submitting) return;
		error = null;
		submitting = true;
		try {
			await authService.login(email.trim(), password);
			await session.load();
			await goto('/dashboard');
		} catch (err) {
			error =
				err instanceof ApiError && err.code === 'AUTH_INVALID_CREDENTIALS'
					? 'E-posta veya şifre hatalı.'
					: 'Giriş yapılamadı. Sunucuya ulaşılamıyor olabilir.';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>Giriş — Canlı Atölye</title>
</svelte:head>

<div class="flex min-h-screen items-center justify-center bg-gray-50 px-4 dark:bg-gray-900">
	<div class="w-full max-w-sm">
		<div class="mb-8 text-center">
			<div
				class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-blue-600 text-lg font-bold text-white"
			>
				CA
			</div>
			<h1 class="text-xl font-semibold text-gray-900 dark:text-white">Canlı Atölye</h1>
			<p class="mt-1 text-sm text-gray-500 dark:text-gray-400">
				Üretim ve Montaj Takip Sistemi
			</p>
		</div>

		<Card class="shadow-sm">
			<form class="flex flex-col gap-4" onsubmit={onSubmit}>
				{#if error}
					<div
						class="rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300"
						role="alert"
					>
						{error}
					</div>
				{/if}

				<div>
					<Label for="email">E-posta</Label>
					<Input
						id="email"
						type="email"
						placeholder="ad@firma.local"
						bind:value={email}
						required
						autocomplete="username"
					/>
				</div>

				<div>
					<Label for="password">Şifre</Label>
					<Input
						id="password"
						type="password"
						placeholder="••••••••"
						bind:value={password}
						required
						autocomplete="current-password"
					/>
				</div>

				<AppButton type="submit" disabled={submitting || email === '' || password === ''}>
					{submitting ? 'Giriş yapılıyor…' : 'Giriş Yap'}
				</AppButton>
			</form>
		</Card>
	</div>
</div>
