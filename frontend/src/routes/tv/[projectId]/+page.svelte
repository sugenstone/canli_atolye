<script lang="ts">
	/**
	 * CANLI FABRİKA / TV MODU (MASTER PLAN §30) — atölye monitöründe sürekli açık.
	 * Tam ekran, koyu tema, dev punto; navigasyon gizli (bu sayfa (app) layout'unun
	 * DIŞINDA durur — sidebar/topbar yok). SSE ile canlı, F ile tam ekran.
	 */
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { insightService, type DashboardSummary, type FlowCard } from '$lib/services/api/insights';
	import { projectService } from '$lib/services/api/projects';
	import { subscribeProject } from '$lib/services/realtime';
	import type { ProjectDto } from '$lib/types';

	const projectId = $derived(page.params.projectId ?? '');

	let project = $state<ProjectDto | null>(null);
	let summary = $state<DashboardSummary | null>(null);
	let activeCards = $state<FlowCard[]>([]);
	let clock = $state(new Date());
	let connectionOk = $state(true);

	async function load() {
		try {
			const [p, s, flow] = await Promise.all([
				projectService.get(projectId),
				insightService.dashboardSummary(projectId),
				insightService.flow(projectId)
			]);
			project = p;
			summary = s;
			// ŞU AN ÇALIŞILIYOR: IN_PROGRESS kartları
			activeCards = flow.filter((c) => c.status === 'IN_PROGRESS');
			connectionOk = true;
		} catch {
			connectionOk = false;
		}
	}

	onMount(() => {
		void load();
		// canlı saat
		const clockTimer = setInterval(() => (clock = new Date()), 1000);
		// SSE + güvenlik yenilemesi
		const sub = subscribeProject(projectId, () => void load());
		const refresh = setInterval(() => void load(), 30_000);
		// F → tam ekran
		const onKey = (e: KeyboardEvent) => {
			if (e.key.toLowerCase() === 'f') {
				if (document.fullscreenElement) void document.exitFullscreen();
				else void document.documentElement.requestFullscreen();
			}
		};
		window.addEventListener('keydown', onKey);
		return () => {
			clearInterval(clockTimer);
			clearInterval(refresh);
			sub.close();
			window.removeEventListener('keydown', onKey);
		};
	});

	const overallPct = $derived.by(() => {
		if (!summary || summary.processes.length === 0) return 0;
		const total = summary.processes.reduce((s, p) => s + p.total, 0);
		const done = summary.processes.reduce((s, p) => s + p.completed, 0);
		return total > 0 ? Math.round((done / total) * 100) : 0;
	});

	const wi = $derived(summary?.work_items);
</script>

<svelte:head>
	<title>{project?.name ?? 'Canlı Fabrika'} — TV Modu</title>
</svelte:head>

<div class="flex min-h-screen flex-col bg-slate-950 p-6 text-white lg:p-10">
	<!-- Başlık: proje + genel ilerleme + saat -->
	<header class="flex items-center justify-between gap-6">
		<div class="flex items-baseline gap-4">
			<h1 class="text-4xl font-black tracking-tight lg:text-5xl">{project?.name ?? '…'}</h1>
			<span class="text-xl font-medium text-slate-500">CANLI FABRİKA</span>
			{#if !connectionOk}
				<span class="rounded bg-red-600 px-2 py-1 text-xs font-bold">BAĞLANTI YOK — YENİDEN DENENİYOR</span>
			{/if}
		</div>
		<div class="flex items-center gap-6">
			<div class="text-right">
				<div class="text-[11px] font-semibold uppercase tracking-widest text-slate-500">Genel İlerleme</div>
				<div class="text-6xl font-black leading-none text-emerald-400 lg:text-7xl">{overallPct}<span class="text-3xl">%</span></div>
			</div>
			<div class="text-right tabular-nums">
				<div class="text-6xl font-bold leading-none lg:text-7xl">
					{clock.toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit' })}
				</div>
				<div class="mt-1 text-sm text-slate-500">
					{clock.toLocaleDateString('tr-TR', { weekday: 'long', day: 'numeric', month: 'long' })}
				</div>
			</div>
		</div>
	</header>

	{#if summary}
		<!-- Süreç ilerleme çubukları -->
		<section class="mt-8 space-y-4">
			{#each summary.processes as p (p.template_id)}
				{@const pct = p.total > 0 ? Math.round((p.completed / p.total) * 100) : 0}
				<div class="flex items-center gap-5">
					<span class="w-44 shrink-0 text-right text-2xl font-bold text-slate-200 lg:w-56">{p.template_name}</span>
					<div class="h-10 flex-1 overflow-hidden rounded-full bg-slate-800 lg:h-12">
						<div
							class="flex h-full items-center justify-end rounded-full bg-gradient-to-r from-emerald-600 to-emerald-400 pr-3 text-sm font-bold text-slate-950 transition-all duration-1000"
							style="width: {Math.max(pct, 5)}%"
						>
							{#if pct >= 8}{pct}%{/if}
						</div>
					</div>
					<span class="w-32 shrink-0 text-xl font-semibold tabular-nums text-slate-400">
						{p.completed}<span class="text-slate-600">/{p.total}}</span>
					</span>
				</div>
			{:else}
				<p class="text-xl text-slate-600">Henüz atanmış süreç yok.</p>
			{/each}
		</section>

		<!-- Alt bölüm: ŞU AN + durum kartları -->
		<div class="mt-auto grid gap-6 pt-10 lg:grid-cols-3">
			<!-- ŞU AN ÇALIŞILIYOR -->
			<section class="lg:col-span-2">
				<h2 class="mb-3 text-sm font-bold uppercase tracking-widest text-sky-400">● Şu An Çalışılıyor ({activeCards.length})</h2>
				<div class="space-y-2">
					{#each activeCards.slice(0, 5) as card (card.execution_id)}
						<div class="flex items-center gap-4 rounded-xl bg-sky-950/60 px-5 py-4 ring-1 ring-sky-900">
							<span class="h-3 w-3 shrink-0 animate-pulse rounded-full bg-sky-400"></span>
							<span class="text-2xl font-bold text-white">{card.work_item_name}</span>
							<span class="truncate text-lg text-slate-400">{card.section_path}</span>
							<span class="ml-auto shrink-0 rounded-lg bg-sky-900 px-3 py-1 text-lg font-bold text-sky-300">{card.template_name}</span>
						</div>
					{:else}
						<div class="rounded-xl bg-slate-900 px-5 py-4 text-xl text-slate-600">Şu anda aktif iş yok.</div>
					{/each}
				</div>
			</section>

			<!-- Durum kartları -->
			<section class="grid grid-cols-2 gap-4">
				<div class="rounded-2xl bg-slate-900 p-5 text-center ring-1 ring-slate-800">
					<div class="text-6xl font-black text-amber-400">{wi?.pending ?? 0}</div>
					<div class="mt-1 text-sm font-semibold uppercase tracking-wide text-slate-500">Bekleyen İş</div>
				</div>
				<div class="rounded-2xl bg-slate-900 p-5 text-center ring-1 ring-slate-800">
					<div class="text-6xl font-black text-emerald-400">{wi?.completed ?? 0}</div>
					<div class="mt-1 text-sm font-semibold uppercase tracking-wide text-slate-500">Tamamlanan</div>
				</div>
				<div class="rounded-2xl p-5 text-center ring-1 {wi && wi.blocked > 0 ? 'bg-red-950/80 ring-red-800' : 'bg-slate-900 ring-slate-800'}">
					<div class="text-6xl font-black text-red-400">{wi?.blocked ?? 0}</div>
					<div class="mt-1 text-sm font-semibold uppercase tracking-wide text-slate-500">⛔ Bloke</div>
				</div>
				<div class="rounded-2xl p-5 text-center ring-1 {wi && wi.late > 0 ? 'bg-orange-950/80 ring-orange-800' : 'bg-slate-900 ring-slate-800'}">
					<div class="text-6xl font-black text-orange-400">{wi?.late ?? 0}</div>
					<div class="mt-1 text-sm font-semibold uppercase tracking-wide text-slate-500">⏰ Geciken</div>
				</div>
				<div class="col-span-2 rounded-2xl bg-emerald-950/60 p-5 text-center ring-1 ring-emerald-900">
					<div class="text-6xl font-black text-emerald-300">{wi?.completed_today ?? 0}</div>
					<div class="mt-1 text-sm font-semibold uppercase tracking-wide text-slate-500">✓ Bugün Tamamlanan</div>
				</div>
			</section>
		</div>
	{:else}
		<div class="flex flex-1 items-center justify-center">
			<div class="h-16 w-16 animate-spin rounded-full border-4 border-slate-800 border-t-emerald-500"></div>
		</div>
	{/if}

	<footer class="pt-6 text-center text-xs text-slate-700">
		Sayfa canlıdır · F tuşu tam ekran yapar · Yönetici: {project?.code ?? ''}
	</footer>
</div>
