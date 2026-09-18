<script lang="ts">
	/**
	 * Recursive section ağaç düğümü — domain component (Flowbite bağımsız).
	 * Eylemler callback prop'ları ile parent sayfaya iletilir; business davranış
	 * burada tutulmaz.
	 */
	import Self from './SectionTreeNode.svelte';
	import type { SectionTreeNode as NodeType } from '$lib/types';
	import { sectionTypeLabel } from '$lib/utils/naming';

	let {
		node,
		depth = 0,
		canManage = false,
		isExpanded,
		onToggle,
		onAddChild,
		onBulkChild,
		onClone,
		onDelete,
		onOpenItems,
		itemCount = 0,
		itemCountById
	}: {
		node: NodeType;
		depth?: number;
		canManage?: boolean;
		isExpanded: (id: string) => boolean;
		onToggle: (id: string) => void;
		onAddChild: (node: NodeType) => void;
		onBulkChild: (node: NodeType) => void;
		onClone: (node: NodeType) => void;
		onDelete: (node: NodeType) => void;
		onOpenItems: (node: NodeType) => void;
		itemCount?: number;
		itemCountById?: (id: string) => number;
	} = $props();

	const childCount = (id: string): number => itemCountById?.(id) ?? 0;

	const expanded = $derived(isExpanded(node.id));
	const typeLabel = $derived(sectionTypeLabel(node.type));
</script>

<div class="flex flex-col">
	<div
		class="group flex items-center gap-1.5 rounded-lg px-2 py-1.5 hover:bg-gray-50 dark:hover:bg-gray-700/30"
		style="padding-left: {depth * 20 + 8}px"
	>
		{#if node.children.length > 0}
			<button
				type="button"
				class="rounded p-0.5 text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600"
				aria-label={expanded ? 'Daralt' : 'Genişlet'}
				onclick={() => onToggle(node.id)}
			>
				<svg
					class="h-4 w-4 transition-transform {expanded ? 'rotate-90' : ''}"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					stroke-width="2"
				>
					<path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
				</svg>
			</button>
		{:else}
			<span class="w-5"></span>
		{/if}

		<button
			type="button"
			class="flex min-w-0 flex-1 items-center gap-2 text-left"
			onclick={() => node.children.length > 0 && onToggle(node.id)}
		>
			<span class="truncate text-sm font-medium text-gray-900 dark:text-white">{node.name}</span>
			{#if node.code}
				<span class="shrink-0 rounded bg-gray-100 px-1.5 py-0.5 text-[10px] font-medium text-gray-500 dark:bg-gray-700 dark:text-gray-300">
					{node.code}
				</span>
			{/if}
			{#if typeLabel}
				<span class="shrink-0 rounded bg-blue-50 px-1.5 py-0.5 text-[10px] font-medium text-blue-700 dark:bg-blue-900/40 dark:text-blue-300">
					{typeLabel}
				</span>
			{/if}
			{#if node.children.length > 0}
				<span class="shrink-0 text-xs text-gray-400">({node.children.length})</span>
			{/if}
		</button>
		{#if itemCount > 0}
			<button
				type="button"
				class="shrink-0 rounded bg-blue-50 px-1.5 py-0.5 text-[10px] font-semibold text-blue-700 hover:bg-blue-100 dark:bg-blue-900/40 dark:text-blue-300 dark:hover:bg-blue-900/60"
				title="İş kalemlerini görüntüle"
				onclick={() => onOpenItems(node)}
			>
				{itemCount} iş
			</button>
		{/if}

		{#if canManage}
			<div class="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
				<button
					type="button"
					class="rounded p-1.5 text-gray-400 hover:bg-blue-50 hover:text-blue-600 dark:hover:bg-blue-900/30"
					title="İş kalemleri"
					aria-label="{node.name} iş kalemleri"
					onclick={() => onOpenItems(node)}
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
					</svg>
				</button>
				<button
					type="button"
					class="rounded p-1.5 text-gray-400 hover:bg-blue-50 hover:text-blue-600 dark:hover:bg-blue-900/30"
					title="Alt bölüm ekle"
					aria-label="{node.name} altına bölüm ekle"
					onclick={() => onAddChild(node)}
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M12 4v16m8-8H4" />
					</svg>
				</button>
				<button
					type="button"
					class="rounded p-1.5 text-gray-400 hover:bg-blue-50 hover:text-blue-600 dark:hover:bg-blue-900/30"
					title="Toplu oluştur (altına)"
					aria-label="{node.name} altına toplu oluştur"
					onclick={() => onBulkChild(node)}
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 10h16M4 14h10M4 18h10" />
					</svg>
				</button>
				<button
					type="button"
					class="rounded p-1.5 text-gray-400 hover:bg-blue-50 hover:text-blue-600 dark:hover:bg-blue-900/30"
					title="Çoğalt (alt ağaçla)"
					aria-label="{node.name} bölümünü çoğalt"
					onclick={() => onClone(node)}
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M8 8V6a2 2 0 012-2h8a2 2 0 012 2v8a2 2 0 01-2 2h-2M6 10h8a2 2 0 012 2v8a2 2 0 01-2 2H6a2 2 0 01-2-2v-8a2 2 0 012-2z"
						/>
					</svg>
				</button>
				<button
					type="button"
					class="rounded p-1.5 text-gray-400 hover:bg-red-50 hover:text-red-600 dark:hover:bg-red-900/30"
					title="Sil"
					aria-label="{node.name} bölümünü sil"
					onclick={() => onDelete(node)}
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
						/>
					</svg>
				</button>
			</div>
		{/if}
	</div>

	{#if expanded && node.children.length > 0}
		{#each node.children as child (child.id)}
			<Self
				node={child}
				depth={depth + 1}
				{canManage}
				{isExpanded}
				{onToggle}
				{onAddChild}
				{onBulkChild}
				{onClone}
				{onDelete}
				{onOpenItems}
				itemCount={childCount(child.id)}
				itemCountById={itemCountById}
			/>
		{/each}
	{/if}
</div>
