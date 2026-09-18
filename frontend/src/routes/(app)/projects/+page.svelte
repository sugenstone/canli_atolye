<script lang="ts">
	/** Proje listesi — satıra tıklayınca bölüm ağacına gidilir. */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import StatusBadge from '$lib/components/ui/StatusBadge.svelte';
	import { projectService } from '$lib/services/api/projects';
	import { ApiError } from '$lib/services/api/client';
	import type { ProjectDto } from '$lib/types';

	let projects = $state<ProjectDto[] | null>(null);
	let loadError = $state<string | null>(null);

	async function load() {
		loadError = null;
		try {
			projects = await projectService.list();
		} catch (err) {
			loadError = err instanceof ApiError ? err.message : 'Projeler yüklenemedi.';
		}
	}

	onMount(load);

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString('tr-TR', {
			day: 'numeric',
			month: 'short',
			year: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>Projeler — Canlı Atölye</title>
</svelte:head>

<div class="mx-auto max-w-6xl space-y-6">
	<div>
		<h1 class="text-xl font-semibold text-gray-900 dark:text-white">Projeler</h1>
		<p class="mt-0.5 text-sm text-gray-500 dark:text-gray-400">
			Proje seçerek bölüm yapısını yönetin.
		</p>
	</div>

	{#if loadError}
		<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
			{loadError}
		</div>
	{:else if projects === null}
		<div class="space-y-2">
			{#each Array(4) as _}
				<div class="h-16 animate-pulse rounded-lg bg-gray-200 dark:bg-gray-700"></div>
			{/each}
		</div>
	{:else if projects.length === 0}
		<div class="rounded-lg border border-gray-200 bg-white px-4 py-12 text-center dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-sm font-medium text-gray-700 dark:text-gray-300">Henüz proje yok.</div>
			<div class="mt-1 text-xs text-gray-500 dark:text-gray-400">
				Dashboard'dan "Yeni Proje" ile ilk projeyi oluşturun.
			</div>
		</div>
	{:else}
		<ul class="divide-y divide-gray-100 overflow-hidden rounded-lg border border-gray-200 bg-white dark:divide-gray-700 dark:border-gray-800 dark:bg-gray-800/50">
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
								{project.code} · {formatDate(project.created_at)}
							</div>
						</div>
						<StatusBadge status={project.status} kind="project" />
						<svg
							class="h-4 w-4 shrink-0 text-gray-400"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
							stroke-width="2"
						>
							<path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
						</svg>
					</a>
				</li>
			{/each}
		</ul>
	{/if}
</div>
