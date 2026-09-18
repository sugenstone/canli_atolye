<script lang="ts">
	/**
	 * Raporlar (MASTER PLAN §36-37): süreç performansı, darboğaz ve takım
	 * performansı. Not (§36): süre verileri işlerin zorluğu farklı olabileceğinden
	 * kişileri cezalandırmak için yorumlanmaz.
	 */
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { api } from '$lib/services/api/client';
	import { ApiError } from '$lib/services/api/client';

	const projectId = $derived(page.params.projectId ?? '');

	interface Row {
		template_id: string;
		template_name: string;
	}
	interface ProcessPerf extends Row {
		completed_count: number;
		avg_active_seconds: number | null;
		avg_waiting_seconds: number | null;
		avg_lead_seconds: number | null;
	}
	interface Bottleneck extends Row {
		ready_count: number;
		in_progress_count: number;
		pending_count: number;
		blocked_count: number;
	}
	interface TeamPerf {
		team_id: string | null;
		team_name: string | null;
		completed_count: number;
		active_seconds_total: number;
	}
	interface Report {
		work_items: {
			total: number;
			completed: number;
			in_progress: number;
			blocked: number;
			pending: number;
			late: number;
		};
		process_performance: ProcessPerf[];
		bottlenecks: Bottleneck[];
		team_performance: TeamPerf[];
	}

	let report = $state<Report | null>(null);
	let loadError = $state<string | null>(null);

	async function load() {
		loadError = null;
		try {
			report = await api.get<Report>(`/projects/${projectId}/reports`);
		} catch (err) {
			loadError = err instanceof ApiError ? err.message : 'Rapor yüklenemedi.';
		}
	}

	onMount(() => void load());

	/** saniye → "1s 23dk" / "45dk" / "2s 05dk" */
	function fmt(sec: number | null): string {
		if (sec === null) return '—';
		if (sec < 60) return `${sec} sn`;
		if (sec < 3600) return `${Math.floor(sec / 60)} dk`;
		return `${Math.floor(sec / 3600)} sa ${String(Math.floor((sec % 3600) / 60)).padStart(2, '0')} dk`;
	}

	// darboğaz yorumu: en yüksek READY yığını belirgin şekilde ortalamaların üzerindeyse
	const bottleneckHint = $derived.by(() => {
		if (!report) return null;
		const rows = report.bottlenecks.filter((b) => b.ready_count > 0);
		if (rows.length === 0) return null;
		const max = Math.max(...rows.map((r) => r.ready_count));
		const avg = rows.reduce((s, r) => s + r.ready_count, 0) / rows.length;
		const top = rows.find((r) => r.ready_count === max);
		if (top && max >= avg * 1.75 && max >= 2) {
			return { name: top.template_name, count: max };
		}
		return null;
	});
</script>

{#if loadError}
	<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
		{loadError}
	</div>
{:else if report === null}
	<div class="space-y-3">
		{#each Array(3) as _}
			<div class="h-24 animate-pulse rounded-lg bg-gray-200 dark:bg-gray-700"></div>
		{/each}
	</div>
{:else}
	<!-- Genel özet -->
	<section class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-800/50">
		<h2 class="mb-3 text-sm font-semibold text-gray-900 dark:text-white">Proje Özeti</h2>
		<div class="grid grid-cols-3 gap-3 sm:grid-cols-6">
			{#each [['Toplam', report.work_items.total, 'text-gray-900 dark:text-white'], ['Tamamlanan', report.work_items.completed, 'text-completed'], ['Devam', report.work_items.in_progress, 'text-in-progress'], ['Bekleyen', report.work_items.pending, 'text-status-pending'], ['Bloke', report.work_items.blocked, 'text-blocked'], ['Geciken', report.work_items.late, 'text-blocked']] as [label, value, cls] (label)}
				<div class="text-center">
					<div class="text-2xl font-semibold {cls}">{value}</div>
					<div class="text-[11px] text-gray-500">{label}</div>
				</div>
			{/each}
		</div>
	</section>

	<!-- Darboğaz -->
	<section class="rounded-lg border {bottleneckHint ? 'border-amber-300 bg-amber-50 dark:border-amber-700 dark:bg-amber-950/20' : 'border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50'} p-4">
		<h2 class="mb-3 text-sm font-semibold text-gray-900 dark:text-white">Darboğaz Analizi</h2>
		{#if bottleneckHint}
			<div class="mb-3 rounded-md border border-amber-400 bg-amber-100 px-3 py-2 text-sm font-medium text-amber-800 dark:border-amber-600 dark:bg-amber-900/40 dark:text-amber-200">
				⚠ Olası darboğaz: {bottleneckHint.name} ({bottleneckHint.count} iş hazır bekliyor)
			</div>
		{/if}
		{#if report.bottlenecks.length === 0}
			<p class="text-sm text-gray-400">Aktif bekleyen süreç yok.</p>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-gray-200 text-left text-xs uppercase tracking-wide text-gray-400 dark:border-gray-700">
							<th class="py-2 pr-3">Süreç</th>
							<th class="px-3 py-2 text-right">Hazır</th>
							<th class="px-3 py-2 text-right">Devam</th>
							<th class="px-3 py-2 text-right">Bekliyor</th>
							<th class="px-3 py-2 text-right">Bloke</th>
						</tr>
					</thead>
					<tbody>
						{#each report.bottlenecks as b (b.template_id)}
							<tr class="border-b border-gray-100 last:border-0 dark:border-gray-700/50">
								<td class="py-2 pr-3 font-medium text-gray-800 dark:text-gray-200">{b.template_name}</td>
								<td class="px-3 py-2 text-right {b.ready_count > 0 ? 'font-bold text-status-ready' : 'text-gray-400'}">{b.ready_count}</td>
								<td class="px-3 py-2 text-right {b.in_progress_count > 0 ? 'text-in-progress' : 'text-gray-400'}">{b.in_progress_count}</td>
								<td class="px-3 py-2 text-right text-gray-400">{b.pending_count}</td>
								<td class="px-3 py-2 text-right {b.blocked_count > 0 ? 'text-blocked' : 'text-gray-400'}">{b.blocked_count}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>

	<!-- Süreç performansı -->
	<section class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-800/50">
		<h2 class="mb-3 text-sm font-semibold text-gray-900 dark:text-white">Süreç Performansı</h2>
		{#if report.process_performance.length === 0}
			<p class="text-sm text-gray-400">Henüz tamamlanmış süreç yok.</p>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-gray-200 text-left text-xs uppercase tracking-wide text-gray-400 dark:border-gray-700">
							<th class="py-2 pr-3">Süreç</th>
							<th class="px-3 py-2 text-right">Tamamlanan</th>
							<th class="px-3 py-2 text-right">Ort. Aktif Süre</th>
							<th class="px-3 py-2 text-right">Ort. Bekleme</th>
							<th class="px-3 py-2 text-right">Ort. Toplam</th>
						</tr>
					</thead>
					<tbody>
						{#each report.process_performance as p (p.template_id)}
							<tr class="border-b border-gray-100 last:border-0 dark:border-gray-700/50">
								<td class="py-2 pr-3 font-medium text-gray-800 dark:text-gray-200">{p.template_name}</td>
								<td class="px-3 py-2 text-right">{p.completed_count}</td>
								<td class="px-3 py-2 text-right font-mono">{fmt(p.avg_active_seconds)}</td>
								<td class="px-3 py-2 text-right font-mono text-gray-500">{fmt(p.avg_waiting_seconds)}</td>
								<td class="px-3 py-2 text-right font-mono">{fmt(p.avg_lead_seconds)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>

	<!-- Takım performansı -->
	<section class="rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-gray-800/50">
		<h2 class="mb-3 text-sm font-semibold text-gray-900 dark:text-white">Takım Performansı</h2>
		<p class="mb-2 text-[11px] text-gray-400">Not: işlerin zorluk dereceleri farklı olabilir; bu veriler kişisel değerlendirme amacı taşımaz.</p>
		{#if report.team_performance.length === 0}
			<p class="text-sm text-gray-400">Takıma atanmış süreç yok.</p>
		{:else}
			<table class="w-full text-sm">
				<thead>
					<tr class="border-b border-gray-200 text-left text-xs uppercase tracking-wide text-gray-400 dark:border-gray-700">
						<th class="py-2 pr-3">Takım</th>
						<th class="px-3 py-2 text-right">Tamamlanan</th>
					</tr>
				</thead>
				<tbody>
					{#each report.team_performance as tp (tp.team_id ?? 'none')}
						<tr class="border-b border-gray-100 last:border-0 dark:border-gray-700/50">
							<td class="py-2 pr-3 font-medium text-gray-800 dark:text-gray-200">{tp.team_name ?? '—'}</td>
							<td class="px-3 py-2 text-right">{tp.completed_count}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</section>
{/if}
