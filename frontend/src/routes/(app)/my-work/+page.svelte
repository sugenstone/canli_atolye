<script lang="ts">
	/**
	 * Benim İşlerim — çalışan ekranı (MASTER PLAN §31, §118).
	 * Mobil-öncelikli: büyük yüksek kontrastlı butonlar, minimum metin,
	 * ana aksiyonlar başparmak erişiminde. Aktif iş en üstte; süre sayacı canlı.
	 */
	import { onMount } from 'svelte';
	import StatusBadge from '$lib/components/ui/StatusBadge.svelte';
	import { api } from '$lib/services/api/client';
	import { blockReasonService, blockService } from '$lib/services/api/operations';
	import { ApiError } from '$lib/services/api/client';
	import { PROCESS_STATUS, statusToken } from '$lib/config/status';

	interface MyWorkCard {
		execution_id: string;
		project_id: string;
		template_name: string;
		work_item_name: string;
		section_path: string;
		status: 'READY' | 'IN_PROGRESS' | 'PAUSED' | 'BLOCKED';
		started_at?: string | null;
		ready_at?: string | null;
		assignment_kind: 'user' | 'team';
		planned_end_at?: string | null;
	}
	const isLate = (c: MyWorkCard) =>
		!!c.planned_end_at && new Date(c.planned_end_at) < new Date() && c.status !== 'BLOCKED';

	let cards = $state<MyWorkCard[] | null>(null);
	let loadError = $state<string | null>(null);
	let busy = $state<string | null>(null); // aksiyon alındaki execution id
	let now = $state(Date.now());
	let reasons = $state<{ id: string; name: string }[]>([]);
	let reportExecId = $state<string | null>(null);
	let reportReason = $state('');

	const GROUPS: { key: MyWorkCard['status'][]; title: string }[] = [
		{ key: ['IN_PROGRESS'], title: 'Devam Eden' },
		{ key: ['PAUSED'], title: 'Duraklatıldı' },
		{ key: ['READY'], title: 'Hazır' },
		{ key: ['BLOCKED'], title: 'Bloke' }
	];

	async function load() {
		loadError = null;
		try {
			cards = await api.get<MyWorkCard[]>('/my-work');
		} catch (err) {
			loadError = err instanceof ApiError ? err.message : 'İşler yüklenemedi.';
		}
	}

	onMount(() => {
		void load();
		void blockReasonService.list().then((r) => (reasons = r)).catch(() => {});
		// süre sayacı
		const timer = setInterval(() => (now = Date.now()), 1000);
		// sayfa odağa dönünce + 30 sn'de bir tazele
		const refresher = setInterval(() => void load(), 30_000);
		const onVisible = () => document.visibilityState === 'visible' && void load();
		document.addEventListener('visibilitychange', onVisible);
		return () => {
			clearInterval(timer);
			clearInterval(refresher);
			document.removeEventListener('visibilitychange', onVisible);
		};
	});

	function elapsed(iso: string): string {
		const s = Math.max(0, Math.floor((now - new Date(iso).getTime()) / 1000));
		const h = Math.floor(s / 3600);
		const m = Math.floor((s % 3600) / 60);
		const sec = s % 60;
		return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`;
	}

	async function act(exec: MyWorkCard, action: 'start' | 'pause' | 'resume' | 'complete') {
		if (busy) return;
		busy = exec.execution_id;
		loadError = null;
		try {
			await api.post(`/process-executions/${exec.execution_id}/${action}`);
			await load();
		} catch (err) {
			loadError = err instanceof ApiError ? err.message : 'İşlem başarısız.';
		} finally {
			busy = null;
		}
	}

	async function submitReport() {
		if (!reportExecId || !reportReason || busy) return;
		busy = reportExecId;
		loadError = null;
		try {
			await blockService.report(reportExecId, reportReason);
			reportExecId = null;
			reportReason = '';
			await load();
		} catch (err) {
			loadError = err instanceof ApiError ? err.message : 'Bildirilemedi.';
		} finally {
			busy = null;
		}
	}

	function groupCards(statuses: MyWorkCard['status'][]): MyWorkCard[] {
		return (cards ?? []).filter((c) => statuses.includes(c.status));
	}
</script>

<svelte:head>
	<title>Benim İşlerim — Canlı Atölye</title>
</svelte:head>

<div class="mx-auto max-w-lg space-y-5">
	<h1 class="text-lg font-semibold text-gray-900 dark:text-white">Benim İşlerim</h1>

	{#if loadError}
		<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
			{loadError}
		</div>
	{/if}

	{#if cards === null}
		<div class="space-y-3">
			{#each Array(3) as _}
				<div class="h-28 animate-pulse rounded-xl bg-gray-200 dark:bg-gray-700"></div>
			{/each}
		</div>
	{:else if cards.length === 0}
		<div class="rounded-xl border border-gray-200 bg-white px-4 py-12 text-center dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-4xl">✓</div>
			<p class="mt-2 text-sm font-medium text-gray-700 dark:text-gray-300">Şu an atanmış işiniz yok.</p>
			<p class="mt-1 text-xs text-gray-400">Yönetici size iş atadığında burada görünecek.</p>
		</div>
	{:else}
		{#each GROUPS as group (group.title)}
			{@const gc = groupCards(group.key)}
			{#if gc.length > 0}
				<section class="space-y-3">
					<h2 class="text-xs font-semibold uppercase tracking-wide text-gray-400">
						{group.title} ({gc.length})
					</h2>
					{#each gc as exec (exec.execution_id)}
						{@const token = statusToken(PROCESS_STATUS, exec.status)}
						<article class="overflow-hidden rounded-xl border border-gray-200 bg-white shadow-sm dark:border-gray-700 dark:bg-gray-800/60">
							<!-- kimlik başlığı -->
							<div class="flex items-center justify-between gap-2 px-4 pt-3.5">
								<div class="min-w-0">
									<div class="truncate text-sm font-bold uppercase tracking-wide text-gray-500 dark:text-gray-400">
										{exec.section_path}
									</div>
									<div class="truncate text-base font-semibold text-gray-900 dark:text-white">
										{exec.work_item_name}
									</div>
								</div>
								<div class="text-right">
									<div class="text-sm font-bold {token.dotClass.replace('bg-', 'text-')}">
										{exec.template_name}
									</div>
									<StatusBadge status={exec.status} kind="process" size="xs" />
									{#if isLate(exec)}
										<div class="mt-0.5 rounded bg-red-100 px-1.5 py-0.5 text-[10px] font-bold text-red-700 dark:bg-red-900/40 dark:text-red-300">
											GECİKİYOR
										</div>
									{/if}
								</div>
							</div>

							<!-- süre sayacı (aktif/paused) -->
							{#if (exec.status === 'IN_PROGRESS' || exec.status === 'PAUSED') && exec.started_at}
								<div class="mt-2 text-center font-mono text-3xl font-bold tabular-nums {exec.status === 'PAUSED' ? 'text-paused' : 'text-in-progress'}">
									{elapsed(exec.started_at)}
								</div>
							{/if}

							<!-- aksiyonlar: büyük, tam genişlik (MASTER PLAN §118) -->
							<div class="mt-3 space-y-2 px-4 pb-4">
								{#if exec.status === 'READY'}
									<button
										type="button"
										class="h-14 w-full rounded-xl bg-blue-600 text-base font-bold text-white transition-colors hover:bg-blue-700 active:bg-blue-800 disabled:opacity-50"
										disabled={busy === exec.execution_id}
										onclick={() => act(exec, 'start')}
									>
										▶ BAŞLAT
									</button>
								{:else if exec.status === 'IN_PROGRESS'}
									<button
										type="button"
										class="h-14 w-full rounded-xl bg-green-600 text-base font-bold text-white transition-colors hover:bg-green-700 active:bg-green-800 disabled:opacity-50"
										disabled={busy === exec.execution_id}
										onclick={() => act(exec, 'complete')}
									>
										✓ İŞİ BİTİR
									</button>
									<button
										type="button"
										class="h-12 w-full rounded-xl border-2 border-orange-500 text-sm font-bold text-orange-600 transition-colors hover:bg-orange-50 active:bg-orange-100 dark:hover:bg-orange-950/30 disabled:opacity-50"
										disabled={busy === exec.execution_id}
										onclick={() => act(exec, 'pause')}
									>
										❙❙ DURAKLAT
									</button>
								{:else if exec.status === 'PAUSED'}
									<button
										type="button"
										class="h-14 w-full rounded-xl bg-blue-600 text-base font-bold text-white transition-colors hover:bg-blue-700 disabled:opacity-50"
										disabled={busy === exec.execution_id}
										onclick={() => act(exec, 'resume')}
									>
										▶ DEVAM ET
									</button>
									<button
										type="button"
										class="h-12 w-full rounded-xl bg-green-600 text-sm font-bold text-white transition-colors hover:bg-green-700 disabled:opacity-50"
										disabled={busy === exec.execution_id}
										onclick={() => act(exec, 'complete')}
									>
										✓ İŞİ BİTİR
									</button>
								{/if}

								{#if exec.status === 'READY' || exec.status === 'IN_PROGRESS'}
									<button
										type="button"
										class="h-10 w-full rounded-xl border border-red-300 text-xs font-semibold text-red-600 transition-colors hover:bg-red-50 dark:border-red-800 dark:hover:bg-red-950/30"
										onclick={() => {
											reportExecId = reportExecId === exec.execution_id ? null : exec.execution_id;
											reportReason = '';
										}}
									>
										⚠ SORUN BİLDİR
									</button>
									{#if reportExecId === exec.execution_id}
										<div class="space-y-2 rounded-lg border border-red-200 bg-red-50/70 p-2.5 dark:border-red-800 dark:bg-red-950/20">
											<select class="h-11 w-full rounded-lg border border-gray-300 bg-white p-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={reportReason}>
												<option value="">Neden seçin…</option>
												{#each reasons as r (r.id)}
													<option value={r.id}>{r.name}</option>
												{/each}
											</select>
											<button
												type="button"
												class="h-11 w-full rounded-lg bg-red-600 text-sm font-bold text-white disabled:opacity-50"
												disabled={!reportReason || busy === exec.execution_id}
												onclick={submitReport}
											>
												BİLDİR
											</button>
										</div>
									{/if}
								{/if}

								{#if exec.assignment_kind === 'team'}
									<div class="pt-0.5 text-center text-[10px] text-gray-400">takım işi</div>
								{/if}
							</div>
						</article>
					{/each}
				</section>
			{/if}
		{/each}
	{/if}
</div>
