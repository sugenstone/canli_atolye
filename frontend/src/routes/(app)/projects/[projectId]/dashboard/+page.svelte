<script lang="ts">
	/** Proje genel bakış: KPI kartları + süreç ilerlemeleri + canlı (SSE). */
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { insightService, type DashboardSummary } from '$lib/services/api/insights';
	import { insightService as insights, bulkPlan } from '$lib/services/api/insights';
	import { subscribeProject } from '$lib/services/realtime';

	let summary = $state<DashboardSummary | null>(null);
	let loadError = $state<string | null>(null);

	const projectId = $derived(page.params.projectId ?? '');

	async function load() {
		loadError = null;
		try {
			summary = await insightService.dashboardSummary(projectId);
		} catch (err) {
			loadError = err instanceof Error ? err.message : 'Özet yüklenemedi.';
		}
	}

	onMount(() => {
		void load();
		const sub = subscribeProject(projectId, () => void load());
		return () => sub.close();
	});

	const wi = $derived(summary?.work_items);

	// Toplu plan aracı (MASTER PLAN §23)
	let planOpen = $state(false);
	let planTemplate = $state('');
	let planDate = $state('');
	let planBusy = $state(false);
	let planResult = $state<string | null>(null);

	async function applyBulkPlan() {
		if (planBusy || !planTemplate || !planDate) return;
		planBusy = true;
		planResult = null;
		try {
			const r = await bulkPlan(projectId, {
				template_id: planTemplate,
				planned_end_at: new Date(planDate).toISOString()
			});
			planResult = `${r.updated} işlem planlandı.`;
			planOpen = false;
			await load();
		} catch {
			planResult = 'Planlama başarısız.';
		} finally {
			planBusy = false;
		}
	}
</script>

{#if loadError}
	<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
		{loadError}
	</div>
{:else if summary === null}
	<div class="grid grid-cols-2 gap-4 lg:grid-cols-6">
		{#each Array(6) as _}
			<div class="h-20 animate-pulse rounded-lg bg-gray-200 dark:bg-gray-700"></div>
		{/each}
	</div>
{:else}
	<!-- KPI kartları -->
	<div class="grid grid-cols-2 gap-3 lg:grid-cols-7">
		<div class="rounded-lg border border-gray-200 bg-white p-3.5 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-[11px] font-medium uppercase tracking-wide text-gray-500">Toplam İş</div>
			<div class="mt-1 text-2xl font-semibold text-gray-900 dark:text-white">{wi?.total}</div>
		</div>
		<div class="rounded-lg border border-gray-200 bg-white p-3.5 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-[11px] font-medium uppercase tracking-wide text-gray-500">Tamamlanan</div>
			<div class="mt-1 text-2xl font-semibold text-completed">{wi?.completed}</div>
		</div>
		<div class="rounded-lg border border-gray-200 bg-white p-3.5 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-[11px] font-medium uppercase tracking-wide text-gray-500">Devam Eden</div>
			<div class="mt-1 text-2xl font-semibold text-in-progress">{wi?.in_progress}</div>
		</div>
		<div class="rounded-lg border border-gray-200 bg-white p-3.5 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-[11px] font-medium uppercase tracking-wide text-gray-500">Bekleyen</div>
			<div class="mt-1 text-2xl font-semibold text-status-pending">{wi?.pending}</div>
		</div>
		<div class="rounded-lg border border-gray-200 bg-white p-3.5 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-[11px] font-medium uppercase tracking-wide text-gray-500">Bloke</div>
			<div class="mt-1 text-2xl font-semibold text-blocked">{wi?.blocked}</div>
		</div>
		<div class="rounded-lg border {wi && wi.late > 0 ? 'border-red-300 bg-red-50 dark:border-red-800 dark:bg-red-950/30' : 'border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50'} p-3.5">
			<div class="text-[11px] font-medium uppercase tracking-wide text-gray-500">Geciken</div>
			<div class="mt-1 text-2xl font-semibold {wi && wi.late > 0 ? 'text-blocked' : 'text-gray-900 dark:text-white'}">{wi?.late}</div>
		</div>
		<div class="rounded-lg border border-gray-200 bg-white p-3.5 dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-[11px] font-medium uppercase tracking-wide text-gray-500">Canlı</div>
			<div class="mt-1 flex items-center gap-1.5 text-sm font-medium text-green-600 dark:text-green-400">
				<span class="relative flex h-2.5 w-2.5">
					<span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-60"></span>
					<span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-green-500"></span>
				</span>
				gerçek zamanlı
			</div>
		</div>
	</div>

	<!-- Süreç ilerlemeleri -->
	<section class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-800/50">
		<div class="mb-3 flex items-center justify-between">
			<h2 class="text-sm font-semibold text-gray-900 dark:text-white">Süreç İlerlemeleri</h2>
			<div class="flex items-center gap-2">
				<a
					href="/tv/{projectId}"
					target="_blank"
					class="rounded-md border border-slate-700 bg-slate-900 px-2.5 py-1 text-xs font-bold text-slate-100 hover:bg-slate-800 dark:border-slate-600"
					title="Atölye monitörü için tam ekran canlı ekran"
				>
					📺 TV Modu
				</a>
				<button type="button" class="rounded-md border border-blue-300 px-2 py-1 text-xs font-medium text-blue-700 hover:bg-blue-50 dark:border-blue-700 dark:text-blue-300 dark:hover:bg-blue-950/30" onclick={() => (planOpen = !planOpen)}>
					Toplu Plan Tarihi
				</button>
			</div>
		</div>
		{#if planOpen}
			<form class="mb-4 flex flex-wrap items-end gap-2 rounded-lg border border-blue-200 bg-blue-50/60 p-3 dark:border-blue-800 dark:bg-blue-950/20" onsubmit={(e) => { e.preventDefault(); void applyBulkPlan(); }}>
				<div>
					<label class="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-300" for="plan-template">Süreç</label>
					<select id="plan-template" class="rounded-md border border-gray-300 bg-white p-1.5 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={planTemplate} required>
						<option value="">Seçin…</option>
						{#each summary.processes as p (p.template_name)}
							<option value={''}>{p.template_name}</option>
						{/each}
					</select>
				</div>
				<div>
					<label class="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-300" for="plan-date">Planlanan bitiş</label>
					<input id="plan-date" type="date" class="rounded-md border border-gray-300 bg-white p-1.5 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={planDate} required />
				</div>
				<button type="submit" class="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-semibold text-white hover:bg-blue-700 disabled:opacity-50" disabled={planBusy || !planTemplate || !planDate}>
					{planBusy ? 'Uygulanıyor…' : 'Uygula'}
				</button>
				{#if planResult}<span class="text-xs text-gray-500">{planResult}</span>{/if}
			</form>
		{/if}
		{#if summary.processes.length === 0}
			<p class="text-sm text-gray-400">Henüz atanmış süreç yok.</p>
		{:else}
			<div class="space-y-4">
				{#each summary.processes as p (p.template_name)}
					{@const pct = p.total > 0 ? Math.round((p.completed / p.total) * 100) : 0}
					<div>
						<div class="flex items-baseline justify-between text-sm">
							<span class="font-medium text-gray-800 dark:text-gray-200">{p.template_name}</span>
							<span class="text-xs text-gray-500">
								{p.completed} / {p.total}
								{#if p.in_progress > 0}<span class="ml-2 text-in-progress">↻ {p.in_progress}</span>{/if}
								{#if p.blocked > 0}<span class="ml-2 text-blocked">⛔ {p.blocked}</span>{/if}
							</span>
						</div>
						<div class="mt-1.5 h-2 overflow-hidden rounded-full bg-gray-100 dark:bg-gray-700">
							<div class="h-full rounded-full bg-completed transition-all duration-500" style="width: {pct}%"></div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</section>
{/if}
