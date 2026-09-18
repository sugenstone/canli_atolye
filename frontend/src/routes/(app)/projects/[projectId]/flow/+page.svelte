<script lang="ts">
	/**
	 * Üretim akışı (MASTER PLAN §29): kolonlar süreç şablonları; kartlar duruma
	 * göre gruplu. Sürükleyerek status değiştirme YOKTUR — business rule backend'de.
	 * SSE ile canlı yenilenir.
	 */
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { insightService, type FlowCard } from '$lib/services/api/insights';
	import { subscribeProject } from '$lib/services/realtime';
	import { PROCESS_STATUS, statusToken } from '$lib/config/status';

	const projectId = $derived(page.params.projectId ?? '');

	let cards = $state<FlowCard[] | null>(null);
	let loadError = $state<string | null>(null);

	async function load() {
		loadError = null;
		try {
			cards = await insightService.flow(projectId);
		} catch (err) {
			loadError = err instanceof Error ? err.message : 'Akış yüklenemedi.';
		}
	}

	onMount(() => {
		void load();
		const sub = subscribeProject(projectId, () => void load());
		return () => sub.close();
	});

	// kolonlar: template adına göre sıralı; her kolonda durum grupları
	const columns = $derived.by(() => {
		if (!cards) return [];
		const map = new Map<string, FlowCard[]>();
		for (const c of cards) {
			const list = map.get(c.template_name) ?? [];
			list.push(c);
			map.set(c.template_name, list);
		}
		return [...map.entries()];
	});

	const STATUS_ORDER = ['READY', 'IN_PROGRESS', 'PAUSED', 'BLOCKED', 'PENDING'];
</script>

{#if loadError}
	<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
		{loadError}
	</div>
{:else if cards === null}
	<div class="grid gap-3 lg:grid-cols-4">
		{#each Array(4) as _}
			<div class="h-64 animate-pulse rounded-lg bg-gray-200 dark:bg-gray-700"></div>
		{/each}
	</div>
{:else if cards.length === 0}
	<div class="rounded-lg border border-gray-200 bg-white px-4 py-10 text-center dark:border-gray-800 dark:bg-gray-800/50">
		<p class="text-sm text-gray-500 dark:text-gray-400">Aktif süreç kartı yok.</p>
		<p class="mt-1 text-xs text-gray-400">Tamamlananlar akışta gösterilmez; süreç atamaları Bölümler sekmesinden yapılır.</p>
	</div>
{:else}
	<div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
		{#each columns as [name, colCards] (name)}
			{@const blocked = colCards.filter((c) => c.status === 'BLOCKED').length}
			<section class="rounded-lg border border-gray-200 bg-gray-50/70 dark:border-gray-800 dark:bg-gray-800/30">
				<header class="flex items-center justify-between border-b border-gray-200 px-3 py-2.5 dark:border-gray-700">
					<h3 class="text-sm font-semibold text-gray-800 dark:text-gray-200">{name}</h3>
					<span class="rounded-full bg-gray-200 px-2 py-0.5 text-[11px] font-semibold text-gray-600 dark:bg-gray-700 dark:text-gray-300">
						{colCards.length}{blocked > 0 ? ` · ⛔${blocked}` : ''}
					</span>
				</header>
				<div class="space-y-1.5 p-2">
					{#each STATUS_ORDER as statusGroup (statusGroup)}
						{@const group = colCards.filter((c) => c.status === statusGroup)}
						{#if group.length > 0}
							<div class="mb-1 px-1 text-[10px] font-semibold uppercase tracking-wide text-gray-400">
								{statusToken(PROCESS_STATUS, statusGroup).label} ({group.length})
							</div>
							{#each group as card (card.execution_id)}
								{@const late = !!card.planned_end_at && new Date(card.planned_end_at) < new Date()}
								<a
									href="/projects/{projectId}/tree"
									class="block rounded-md border bg-white px-2.5 py-2 transition-colors hover:border-blue-300 dark:bg-gray-800 dark:hover:border-blue-600 {late ? 'border-red-300 dark:border-red-700' : 'border-gray-200 dark:border-gray-700'}"
									title="{card.section_path} — {card.work_item_name}{late ? ' (GECİKİYOR)' : ''}"
								>
									<div class="flex items-center gap-1.5">
										<span class="h-2 w-2 shrink-0 rounded-full {statusToken(PROCESS_STATUS, card.status).dotClass}"></span>
										<span class="truncate text-xs font-medium text-gray-900 dark:text-white">{card.work_item_name}</span>
										{#if late}<span class="text-[10px] text-blocked">⏰</span>{/if}
									</div>
									<div class="mt-0.5 truncate text-[10px] text-gray-400">{card.section_path}</div>
								</a>
							{/each}
						{/if}
					{/each}
				</div>
			</section>
		{/each}
	</div>
{/if}
