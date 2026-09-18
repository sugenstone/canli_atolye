<script lang="ts">
	/**
	 * Kat/daire matrisi (MASTER PLAN §27, §69): satır = üst bölümün çocukları,
	 * sütun = pozisyon. Renk + ikon + tooltip birlikte (renk körlüğü erişilebilirliği).
	 * Hücreye tıkla → o bölümün iş kalemleri drawer'ı. SSE ile canlı yenilenir.
	 */
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { Drawer } from 'flowbite-svelte';
	import StatusBadge from '$lib/components/ui/StatusBadge.svelte';
	import WorkItemsDrawer from '$lib/components/domain/WorkItemsDrawer.svelte';
	import { insightService, type MatrixResponse } from '$lib/services/api/insights';
	import { sectionService } from '$lib/services/api/sections';
	import { subscribeProject } from '$lib/services/realtime';
	import { session } from '$lib/stores/session.svelte';
	import { PROCESS_STATUS, statusToken } from '$lib/config/status';
	import type { SectionTreeNode } from '$lib/types';

	const projectId = $derived(page.params.projectId ?? '');

	let tree = $state<SectionTreeNode[] | null>(null);
	let parentId = $state('');
	let matrix = $state<MatrixResponse | null>(null);
	let loadError = $state<string | null>(null);

	// hücre drawer'ı
	let drawerOpen = $state(false);
	let drawerSection = $state<SectionTreeNode | null>(null);

	const canManage = $derived(session.isAdmin || session.me?.user.role === 'PROJECT_MANAGER');

	/** Çocuğu olan bölümler (potansiyel matris üstü). */
	function flattenWithChildren(nodes: SectionTreeNode[], depth = 0): { node: SectionTreeNode; depth: number }[] {
		const out: { node: SectionTreeNode; depth: number }[] = [];
		for (const n of nodes) {
			if (n.children.length > 0) out.push({ node: n, depth });
			out.push(...flattenWithChildren(n.children, depth + 1));
		}
		return out;
	}

	const parents = $derived(flattenWithChildren(tree ?? []));

	async function loadMatrix() {
		if (!parentId) return;
		loadError = null;
		try {
			matrix = await insightService.matrix(projectId, parentId);
		} catch (err) {
			loadError = err instanceof Error ? err.message : 'Matris yüklenemedi.';
		}
	}

	async function init() {
		try {
			const treeData = await sectionService.tree(projectId);
			tree = treeData.nodes;
			// varsayılan: en üst seviyede çocuklu ilk bölüm (ör. A Blok)
			if (!parentId) {
				const candidate = parents[0]?.node.id ?? '';
				parentId = candidate;
			}
			if (parentId) await loadMatrix();
		} catch (err) {
			loadError = err instanceof Error ? err.message : 'Ağaç yüklenemedi.';
		}
	}

	onMount(() => {
		void init();
		const sub = subscribeProject(projectId, () => void loadMatrix());
		return () => sub.close();
	});

	function openCell(sectionId: string, name: string) {
		drawerSection = {
			id: sectionId,
			name,
			project_id: projectId,
			parent_id: null,
			code: null,
			type: null,
			sort_order: 0,
			created_at: '',
			updated_at: '',
			children: []
		};
		drawerOpen = true;
	}

	function cellOf(rowId: string, position: number) {
		return matrix?.cells[rowId]?.[String(position)] ?? null;
	}
</script>

<div class="space-y-4">
	<div class="flex flex-wrap items-center gap-3">
		<label for="matrix-parent" class="text-sm font-medium text-gray-700 dark:text-gray-300">Matris üstü:</label>
		<select
			id="matrix-parent"
			class="rounded-lg border border-gray-300 bg-white px-2.5 py-1.5 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white"
			bind:value={parentId}
			onchange={() => void loadMatrix()}
		>
			{#each parents as { node, depth } (node.id)}
				<option value={node.id}>{' '.repeat(depth * 2)}{node.name}</option>
			{/each}
		</select>
		{#if matrix}
			<span class="text-xs text-gray-400">{matrix.rows.length} satır × {matrix.cols.length} sütun</span>
		{/if}
	</div>

	{#if loadError}
		<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
			{loadError}
		</div>
	{:else if parents.length === 0}
		<div class="rounded-lg border border-gray-200 bg-white px-4 py-10 text-center dark:border-gray-800 dark:bg-gray-800/50">
			<p class="text-sm text-gray-500 dark:text-gray-400">
				Matris için en az üç seviye gerekli (ör. Blok → Kat → Daire).
				<a href="/projects/{projectId}/tree" class="font-medium text-blue-600 hover:underline">Bölümler</a>
				sekmesinden yapı kurun.
			</p>
		</div>
	{:else if matrix === null}
		<div class="space-y-2">
			{#each Array(5) as _}
				<div class="h-10 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
			{/each}
		</div>
	{:else}
		<div class="overflow-x-auto rounded-lg border border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50">
			<table class="w-full border-collapse text-sm">
				<thead>
					<tr class="border-b border-gray-200 dark:border-gray-700">
						<th class="sticky left-0 z-10 bg-white px-3 py-2 text-left text-xs font-semibold uppercase tracking-wide text-gray-400 dark:bg-gray-800"></th>
						{#each matrix.cols as col (col.position)}
							<th class="px-2 py-2 text-center text-xs font-semibold text-gray-600 dark:text-gray-300">
								{col.label}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each matrix.rows as row (row.id)}
						<tr class="border-b border-gray-100 last:border-0 dark:border-gray-700/50">
							<th class="sticky left-0 z-10 bg-white px-3 py-2 text-left text-xs font-semibold text-gray-700 dark:bg-gray-800 dark:text-gray-200">
								{row.name}
							</th>
							{#each matrix.cols as col (col.position)}
								{@const cell = cellOf(row.id, col.position)}
								<td class="px-1.5 py-1.5 text-center">
									{#if cell}
										<button
											type="button"
											class="group inline-flex min-w-16 flex-col items-center gap-0.5 rounded-lg px-2 py-2 transition-transform hover:scale-105 {statusToken(PROCESS_STATUS, cell.status).dotClass}/15 hover:{statusToken(PROCESS_STATUS, cell.status).dotClass}/25"
											title="{row.name} / {col.label}: {statusToken(PROCESS_STATUS, cell.status).label} — {cell.label}"
											onclick={() => openCell(cell.section_id, `${row.name} / ${col.label}`)}
										>
											<span class="h-3 w-3 rounded-full {statusToken(PROCESS_STATUS, cell.status).dotClass}"></span>
											<span class="text-[10px] font-medium {statusToken(PROCESS_STATUS, cell.status).dotClass.replace('bg-', 'text-')}">
												{statusToken(PROCESS_STATUS, cell.status).label}
											</span>
										</button>
									{:else}
										<span class="text-xs text-gray-300 dark:text-gray-600">—</span>
									{/if}
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Lejant (renk + metin birlikte) -->
		<div class="flex flex-wrap gap-3 text-xs text-gray-500 dark:text-gray-400">
			{#each Object.entries(PROCESS_STATUS) as [key, token]}
				<span class="inline-flex items-center gap-1.5">
					<span class="h-2.5 w-2.5 rounded-full {token.dotClass}"></span>{token.label}
				</span>
			{/each}
		</div>
	{/if}
</div>

<WorkItemsDrawer bind:open={drawerOpen} section={drawerSection} {projectId} {canManage} isAdmin={session.isAdmin} />
