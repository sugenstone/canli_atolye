<script lang="ts">
	/** Aktivite akışı (MASTER PLAN §38): projenin son süreç olayları. SSE ile canlı. */
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { insightService, type ActivityRow } from '$lib/services/api/insights';
	import { subscribeProject } from '$lib/services/realtime';

	const projectId = $derived(page.params.projectId ?? '');

	let rows = $state<ActivityRow[] | null>(null);
	let loadError = $state<string | null>(null);

	async function load() {
		loadError = null;
		try {
			rows = await insightService.activity(projectId, 60);
		} catch (err) {
			loadError = err instanceof Error ? err.message : 'Aktivite yüklenemedi.';
		}
	}

	onMount(() => {
		void load();
		const sub = subscribeProject(projectId, () => void load());
		return () => sub.close();
	});

	const EVENT_LABELS: Record<string, string> = {
		CREATED: 'oluşturuldu',
		READY: 'hazır hale geldi',
		ASSIGNED: 'atandı',
		STARTED: 'başlatıldı',
		PAUSED: 'duraklatıldı',
		RESUMED: 'devam ettirildi',
		BLOCKED: 'bloke edildi',
		UNBLOCKED: 'blokesi kaldırıldı',
		COMPLETED: 'tamamlandı',
		CANCELLED: 'iptal edildi',
		NOTE_ADDED: 'not eklendi',
		FILE_ADDED: 'dosya eklendi'
	};

	function timeOf(iso: string): string {
		return new Date(iso).toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit' });
	}
	function dateOf(iso: string): string {
		return new Date(iso).toLocaleDateString('tr-TR', { day: 'numeric', month: 'short' });
	}
</script>

{#if loadError}
	<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
		{loadError}
	</div>
{:else if rows === null}
	<div class="space-y-2">
		{#each Array(8) as _}
			<div class="h-10 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
		{/each}
	</div>
{:else if rows.length === 0}
	<div class="rounded-lg border border-gray-200 bg-white px-4 py-10 text-center dark:border-gray-800 dark:bg-gray-800/50">
		<p class="text-sm text-gray-500 dark:text-gray-400">Henüz aktivite yok.</p>
	</div>
{:else}
	<ol class="relative ml-3 border-l border-gray-200 dark:border-gray-700">
		{#each rows as row (row.timestamp + row.event_type + row.work_item_name)}
			<li class="mb-4 ml-4">
				<span class="absolute -left-[5px] mt-1.5 h-2.5 w-2.5 rounded-full bg-blue-500"></span>
				<time class="mb-0.5 block text-[10px] font-normal leading-none text-gray-400">
					{dateOf(row.timestamp)} {timeOf(row.timestamp)}
				</time>
				<div class="text-sm text-gray-700 dark:text-gray-300">
					<span class="font-semibold">{row.user_name}</span>
					— {row.work_item_name}
					<span class="text-gray-500 dark:text-gray-400">{EVENT_LABELS[row.event_type] ?? row.event_type.toLowerCase()}</span>
				</div>
				<div class="text-xs text-gray-400">
					{row.section_path}
					{#if row.note}<span class="ml-1 italic">“{row.note}”</span>{/if}
				</div>
			</li>
		{/each}
	</ol>
{/if}
