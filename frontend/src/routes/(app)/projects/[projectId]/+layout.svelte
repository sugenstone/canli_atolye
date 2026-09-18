<script lang="ts">
	/** Proje alt navigasyonu (MASTER PLAN §48): sekmeler + breadcrumb. */
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { projectService } from '$lib/services/api/projects';
	import type { ProjectDto } from '$lib/types';

	let { children } = $props();

	const projectId = $derived(page.params.projectId ?? '');

	const tabs = [
		{ href: 'dashboard', label: 'Genel Bakış' },
		{ href: 'tree', label: 'Bölümler' },
		{ href: 'matrix', label: 'Matris' },
		{ href: 'flow', label: 'Üretim Akışı' },
		{ href: 'activity', label: 'Aktiviteler' },
		{ href: 'reports', label: 'Raporlar' }
	];

	let project = $state<ProjectDto | null>(null);

	$effect(() => {
		if (projectId) {
			void projectService.get(projectId).then((p) => (project = p)).catch(() => {});
		}
	});

	const currentTab = $derived(page.url.pathname.split('/').pop() ?? 'dashboard');
</script>

<div class="mx-auto max-w-7xl space-y-4">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div class="min-w-0">
			<a href="/projects" class="text-xs font-medium text-gray-400 hover:text-blue-600">← Projeler</a>
			<h1 class="mt-0.5 truncate text-xl font-semibold text-gray-900 dark:text-white">
				{project?.name ?? '…'}
				<span class="ml-2 text-sm font-normal text-gray-400">{project?.code}</span>
			</h1>
		</div>
	</div>

	<nav class="flex gap-1 overflow-x-auto border-b border-gray-200 dark:border-gray-700" aria-label="Proje sekmeleri">
		{#each tabs as tab (tab.href)}
			<a
				href="/projects/{projectId}/{tab.href}"
				class="whitespace-nowrap border-b-2 px-3.5 py-2.5 text-sm font-medium transition-colors
					{currentTab === tab.href
						? 'border-blue-600 text-blue-700 dark:border-blue-400 dark:text-blue-300'
						: 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
			>
				{tab.label}
			</a>
		{/each}
	</nav>

	{@render children?.()}
</div>
