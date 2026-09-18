<script lang="ts">
	/**
	 * Faz 1 dashboard: oturum özeti + proje listesi.
	 * KPI kartları Faz 6'da (dashboard aggregates) gerçek veriyle dolacak;
	 * süreç kartları Faz 4 sonrası görünür olacak.
	 */
	import { onMount } from 'svelte';
	import { Input, Label } from 'flowbite-svelte';
	import AppButton from '$lib/components/ui/AppButton.svelte';
	import AppModal from '$lib/components/ui/AppModal.svelte';
	import StatusBadge from '$lib/components/ui/StatusBadge.svelte';
	import { projectService } from '$lib/services/api/projects';
	import { ApiError } from '$lib/services/api/client';
	import { session } from '$lib/stores/session.svelte';
	import type { ProjectDto } from '$lib/types';

	let projects = $state<ProjectDto[] | null>(null);
	let loadError = $state<string | null>(null);

	// Yeni proje modalı (ADMIN)
	let modalOpen = $state(false);
	let newName = $state('');
	let newCode = $state('');
	let creating = $state(false);
	let createError = $state<string | null>(null);
	let createFormEl = $state<HTMLFormElement | undefined>(undefined);

	const isAdmin = $derived(session.isAdmin);

	async function loadProjects() {
		loadError = null;
		try {
			projects = await projectService.list();
		} catch (err) {
			loadError =
				err instanceof ApiError ? err.message : 'Projeler yüklenemedi.';
		}
	}

	onMount(loadProjects);

	async function onCreate(e: SubmitEvent) {
		e.preventDefault();
		if (creating || !newName.trim() || !newCode.trim()) return;
		creating = true;
		createError = null;
		try {
			await projectService.create({
				name: newName.trim(),
				code: newCode.trim(),
				description: undefined
			});
			modalOpen = false;
			newName = '';
			newCode = '';
			await loadProjects();
		} catch (err) {
			createError = err instanceof ApiError ? err.message : 'Proje oluşturulamadı.';
		} finally {
			creating = false;
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString('tr-TR', {
			day: 'numeric',
			month: 'short',
			year: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>Dashboard — Canlı Atölye</title>
</svelte:head>

<div class="mx-auto max-w-6xl space-y-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h1 class="text-xl font-semibold text-gray-900 dark:text-white">Genel Bakış</h1>
			<p class="mt-0.5 text-sm text-gray-500 dark:text-gray-400">
				Merhaba {session.me?.user.full_name ?? ''} — {session.me?.workspace.name ?? ''}
			</p>
		</div>
		{#if isAdmin}
			<AppButton onclick={() => (modalOpen = true)}>+ Yeni Proje</AppButton>
		{/if}
	</div>

	<!-- KPI kartları: Faz 1'de proje sayıları; süreç KPI'ları Faz 6'da -->
	<div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
		<div class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-xs font-medium uppercase tracking-wide text-gray-500 dark:text-gray-400">Toplam Proje</div>
			<div class="mt-1 text-2xl font-semibold text-gray-900 dark:text-white">
				{projects?.length ?? '—'}
			</div>
		</div>
		<div class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-xs font-medium uppercase tracking-wide text-gray-500 dark:text-gray-400">Aktif Proje</div>
			<div class="mt-1 text-2xl font-semibold text-in-progress">
				{projects ? projects.filter((p) => p.status === 'ACTIVE').length : '—'}
			</div>
		</div>
		<div class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-xs font-medium uppercase tracking-wide text-gray-500 dark:text-gray-400">Tamamlanan</div>
			<div class="mt-1 text-2xl font-semibold text-completed">
				{projects ? projects.filter((p) => p.status === 'COMPLETED').length : '—'}
			</div>
		</div>
		<div class="rounded-lg border border-dashed border-gray-300 bg-gray-50 p-4 dark:border-gray-700 dark:bg-gray-800/30">
			<div class="text-xs font-medium uppercase tracking-wide text-gray-400 dark:text-gray-500">Süreç KPI'ları</div>
			<div class="mt-1 text-sm text-gray-400 dark:text-gray-500">Süreç motoruyla birlikte aktifleşecek (Faz 4+)</div>
		</div>
	</div>

	<!-- Proje listesi -->
	<section class="rounded-lg border border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50">
		<header class="flex items-center justify-between border-b border-gray-200 px-4 py-3 dark:border-gray-700">
			<h2 class="text-sm font-semibold text-gray-900 dark:text-white">Projeler</h2>
			<button
				type="button"
				class="text-xs font-medium text-blue-600 hover:underline dark:text-blue-400"
				onclick={loadProjects}
			>
			Yenile
			</button>
		</header>

		{#if loadError}
			<div class="px-4 py-6 text-center text-sm text-red-600 dark:text-red-400">{loadError}</div>
		{:else if projects === null}
			<!-- Skeleton satırlar -->
			<div class="divide-y divide-gray-100 dark:divide-gray-700">
				{#each Array(3) as _}
					<div class="flex items-center gap-4 px-4 py-3.5">
						<div class="h-4 w-40 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
						<div class="h-4 w-16 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
						<div class="ms-auto h-5 w-24 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
					</div>
				{/each}
			</div>
		{:else if projects.length === 0}
			<!-- Empty state (MASTER PLAN §52) -->
			<div class="flex flex-col items-center gap-2 px-4 py-10 text-center">
				<div class="text-sm font-medium text-gray-700 dark:text-gray-300">Henüz proje yok.</div>
				<div class="text-xs text-gray-500 dark:text-gray-400">
					{isAdmin ? 'İlk projeyi oluşturarak başlayın.' : 'Yöneticiniz bir proje oluşturduğunda burada görünecek.'}
				</div>
				{#if isAdmin}
					<div class="mt-2"><AppButton size="sm" onclick={() => (modalOpen = true)}>Proje Oluştur</AppButton></div>
				{/if}
			</div>
		{:else}
			<ul class="divide-y divide-gray-100 dark:divide-gray-700">
				{#each projects as project (project.id)}
					<li>
						<a
							href="/projects/{project.id}/tree"
							class="flex flex-wrap items-center gap-x-4 gap-y-1.5 px-4 py-3.5 hover:bg-gray-50 dark:hover:bg-gray-700/30"
						>
							<div class="min-w-0 flex-1">
								<div class="truncate text-sm font-medium text-gray-900 dark:text-white">
									{project.name}
								</div>
								<div class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
									{project.code} · {formatDate(project.created_at)} tarihinde oluşturuldu
								</div>
							</div>
							<StatusBadge status={project.status} kind="project" />
						</a>
					</li>
				{/each}
			</ul>
		{/if}
	</section>
</div>

<AppModal bind:open={modalOpen} title="Yeni Proje">
	<form id="create-project" bind:this={createFormEl} class="flex flex-col gap-4" onsubmit={onCreate}>
		{#if createError}
			<div class="rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
				{createError}
			</div>
		{/if}
		<div>
			<Label for="project-name">Proje Adı</Label>
			<Input id="project-name" placeholder="Gardenia" bind:value={newName} required />
		</div>
		<div>
			<Label for="project-code">Proje Kodu</Label>
			<Input id="project-code" placeholder="GRD" bind:value={newCode} required />
		</div>
	</form>
	{#snippet footer()}
		<AppButton color="alternative" onclick={() => (modalOpen = false)}>İptal</AppButton>
		<AppButton
			disabled={creating || !newName.trim() || !newCode.trim()}
			onclick={() => createFormEl?.requestSubmit()}
		>
			{creating ? 'Oluşturuluyor…' : 'Oluştur'}
		</AppButton>
	{/snippet}
</AppModal>
