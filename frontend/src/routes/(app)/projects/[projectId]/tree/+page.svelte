<script lang="ts">
	/**
	 * Bölüm ağacı ekranı (MASTER PLAN §24, §27 temeli):
	 * ağaç + tekli ekleme + bulk üretici ("Daire 1..10") + alt ağaç çoğaltma
	 * ("katı ×14") + silme (confirm). Yönetim eylemleri ADMIN/PM'ye özeldir.
	 */
	import { page } from '$app/state';
	import { Input, Label, Select } from 'flowbite-svelte';
	import AppButton from '$lib/components/ui/AppButton.svelte';
	import AppModal from '$lib/components/ui/AppModal.svelte';
	import SectionTreeNode from '$lib/components/domain/SectionTreeNode.svelte';
	import WorkItemsDrawer from '$lib/components/domain/WorkItemsDrawer.svelte';
	import { workItemService, workItemTypeService } from '$lib/services/api/workItems';
	import { sectionService } from '$lib/services/api/sections';
	import { projectService } from '$lib/services/api/projects';
	import { ApiError } from '$lib/services/api/client';
	import { session } from '$lib/stores/session.svelte';
	import { deriveCloneName } from '$lib/utils/naming';
	import type { ProjectDto, SectionTreeNode as NodeType, WorkItemTypeDto } from '$lib/types';

	// Route parametresi: SvelteKit'te $page.params üzerinden alınır
	const projectId = $derived(page.params.projectId ?? '');

	let project = $state<ProjectDto | null>(null);
	let tree = $state<NodeType[] | null>(null);
	let total = $state(0);
	let loadError = $state<string | null>(null);
	let busy = $state(false);

	// İş kalemleri: bölüm bazlı sayaçlar + drawer + bulk modal
	let itemCounts = $state<Record<string, number>>({});
	let types = $state<WorkItemTypeDto[]>([]);
	let drawerOpen = $state(false);
	let selectedSection = $state<NodeType | null>(null);
	let bulkItemsOpen = $state(false);
	let bulkItemsParent = $state('');
	let bulkItemsTypeId = $state('');
	let bulkItemsBusy = $state(false);
	let bulkItemsError = $state<string | null>(null);
	let bulkItemsFormEl = $state<HTMLFormElement | undefined>(undefined);

	function countItems(id: string): number {
		return itemCounts[id] ?? 0;
	}

	const canManage = $derived(session.isAdmin || session.me?.user.role === 'PROJECT_MANAGER');

	// Ağaç genişletme durumu — immutable obje (Set yerine: callback prop'lar
	// üzerinden güvenilir reaktivite için her değişimde yeni referans)
	let expanded = $state<Record<string, boolean>>({});

	function setExpanded(id: string, value: boolean) {
		expanded = { ...expanded, [id]: value };
	}

	function toggle(id: string) {
		setExpanded(id, !expanded[id]);
	}

	async function loadTree() {
		loadError = null;
		try {
			const [projectData, treeData] = await Promise.all([
				projectService.get(projectId),
				sectionService.tree(projectId)
            ]);
			project = projectData;
			tree = treeData.nodes;
			total = treeData.total;
			// iş kalemi sayaçları + tipler (paralel, hata akışı bozmaz)
			try {
				const [allItems, typeList] = await Promise.all([
					workItemService.listAll(projectId),
					workItemTypeService.list()
				]);
				const counts: Record<string, number> = {};
				for (const item of allItems) {
					counts[item.section_id] = (counts[item.section_id] ?? 0) + 1;
				}
				itemCounts = counts;
				types = typeList;
			} catch {
				itemCounts = {};
			}
			// İlk seviyeyi otomatik genişlet
			if (Object.keys(expanded).length === 0) {
				expanded = Object.fromEntries(treeData.nodes.map((n) => [n.id, true]));
			}
		} catch (err) {
			loadError = err instanceof ApiError ? err.message : 'Ağaç yüklenemedi.';
		}
	}

	// projectId değişirse (farklı projeye navigasyon) ağacı yeniden yükle
	$effect(() => {
		if (projectId) {
			void loadTree();
		}
	});

	// --- Tek bölüm ekleme ---
	let createOpen = $state(false);
	let createParent = $state<NodeType | null>(null); // null → kök seviye
	let createName = $state('');
	let createCode = $state('');
	let createType = $state('');
	let createError = $state<string | null>(null);
	let createFormEl = $state<HTMLFormElement | undefined>(undefined);

	function openCreate(parent: NodeType | null) {
		createParent = parent;
		createName = '';
		createCode = '';
		createType = '';
		createError = null;
		createOpen = true;
	}

	async function submitCreate(e: SubmitEvent) {
		e.preventDefault();
		if (busy) return;
		busy = true;
		createError = null;
		try {
			await sectionService.create(projectId, {
				name: createName.trim(),
				code: createCode.trim() || undefined,
				type: createType.trim() || undefined,
				parent_id: createParent?.id ?? null
			});
			createOpen = false;
			if (createParent) setExpanded(createParent.id, true);
			await loadTree();
		} catch (err) {
			createError = err instanceof ApiError ? err.message : 'Bölüm eklenemedi.';
		} finally {
			busy = false;
		}
	}

	// --- Toplu oluşturma ---
	let bulkOpen = $state(false);
	let bulkParent = $state<NodeType | null>(null);
	let bulkStart = $state(1);
	let bulkCount = $state(10);
	let bulkNameFormat = $state('Daire {n}');
	let bulkCodeFormat = $state('');
	let bulkType = $state('');
	let bulkError = $state<string | null>(null);
	let bulkFormEl = $state<HTMLFormElement | undefined>(undefined);

	function openBulk(parent: NodeType | null) {
		bulkParent = parent;
		bulkStart = 1;
		bulkCount = 10;
		bulkNameFormat = 'Daire {n}';
		bulkCodeFormat = '';
		bulkType = '';
		bulkError = null;
		bulkOpen = true;
	}

	async function submitBulk(e: SubmitEvent) {
		e.preventDefault();
		if (busy) return;
		busy = true;
		bulkError = null;
		try {
			await sectionService.bulkCreate(projectId, {
				parent_id: bulkParent?.id ?? null,
				start_index: bulkStart,
				count: bulkCount,
				name_format: bulkNameFormat,
				code_format: bulkCodeFormat.trim() || undefined,
				type: bulkType.trim() || undefined
			});
			bulkOpen = false;
			if (bulkParent) setExpanded(bulkParent.id, true);
			await loadTree();
		} catch (err) {
			bulkError = err instanceof ApiError ? err.message : 'Toplu oluşturma başarısız.';
		} finally {
			busy = false;
		}
	}

	// --- Çoğaltma ---
	let cloneOpen = $state(false);
	let cloneNode = $state<NodeType | null>(null);
	let cloneCount = $state(1);
	let cloneNameFormat = $state(''); // boş → otomatik türetme
	let cloneError = $state<string | null>(null);
	let cloneFormEl = $state<HTMLFormElement | undefined>(undefined);

	const clonePreview = $derived.by(() => {
		const node = cloneNode;
		if (!node) return '';
		const names = [1, 2, 3]
			.map((k) =>
				cloneNameFormat.includes('{n}')
					? cloneNameFormat.replace('{n}', String(k))
					: deriveCloneName(node.name, k)
			)
			.slice(0, Math.min(3, cloneCount));
		return names.join(', ') + (cloneCount > 3 ? ', …' : '');
	});

	function openClone(node: NodeType) {
		cloneNode = node;
		cloneCount = 1;
		cloneNameFormat = '';
		cloneError = null;
		cloneOpen = true;
	}

	async function submitClone(e: SubmitEvent) {
		e.preventDefault();
		if (busy || !cloneNode) return;
		busy = true;
		cloneError = null;
		try {
			await sectionService.clone(cloneNode.id, {
				count: cloneCount,
				name_format: cloneNameFormat.trim() || undefined
			});
			cloneOpen = false;
			await loadTree();
		} catch (err) {
			cloneError = err instanceof ApiError ? err.message : 'Çoğaltma başarısız.';
		} finally {
			busy = false;
		}
	}

	// --- Silme (confirm'lu kritik aksiyon, MASTER PLAN §55) ---
	let deleteOpen = $state(false);
	let deleteNode = $state<NodeType | null>(null);
	let deleteError = $state<string | null>(null);

	function openDelete(node: NodeType) {
		deleteNode = node;
		deleteError = null;
		deleteOpen = true;
	}

	function openItems(node: NodeType) {
		selectedSection = node;
		drawerOpen = true;
	}

	function flatten(nodes: NodeType[], depth = 0): { node: NodeType; depth: number }[] {
		return nodes.flatMap((n) => [{ node: n, depth }, ...flatten(n.children, depth + 1)]);
	}

	function openBulkItems() {
		bulkItemsParent = '';
		bulkItemsTypeId = '';
		bulkItemsError = null;
		bulkItemsOpen = true;
	}

	async function submitBulkItems(e: SubmitEvent) {
		e.preventDefault();
		if (bulkItemsBusy || !bulkItemsParent || !bulkItemsTypeId) return;
		bulkItemsBusy = true;
		bulkItemsError = null;
		try {
			await workItemService.bulkCreate(projectId, bulkItemsParent, bulkItemsTypeId);
			bulkItemsOpen = false;
			await loadTree();
		} catch (err) {
			bulkItemsError = err instanceof ApiError ? err.message : 'Toplu ekleme başarısız.';
		} finally {
			bulkItemsBusy = false;
		}
	}

	async function confirmDelete() {
		if (busy || !deleteNode) return;
		busy = true;
		deleteError = null;
		try {
			await sectionService.remove(deleteNode.id);
			deleteOpen = false;
			await loadTree();
		} catch (err) {
			deleteError = err instanceof ApiError ? err.message : 'Silme başarısız.';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head>
	<title>{project?.name ?? 'Proje'} · Bölümler — Canlı Atölye</title>
</svelte:head>

<div class="mx-auto max-w-5xl space-y-5">
	<!-- Başlık -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<p class="text-sm text-gray-500 dark:text-gray-400">
			{total} bölüm · hiyerarşik yapı
		</p>
		{#if canManage}
			<div class="flex flex-wrap gap-2">
				<AppButton color="alternative" onclick={openBulkItems}>Yapraklara İş Kalemi</AppButton>
				<AppButton color="alternative" onclick={() => openBulk(null)}>Toplu Bölüm</AppButton>
				<AppButton onclick={() => openCreate(null)}>+ Bölüm Ekle</AppButton>
			</div>
		{/if}
	</div>

	{#if loadError}
		<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
			{loadError}
		</div>
	{:else if tree === null}
		<div class="space-y-2">
			{#each Array(5) as _}
				<div class="h-9 animate-pulse rounded-lg bg-gray-200 dark:bg-gray-700"></div>
			{/each}
		</div>
	{:else if tree.length === 0}
		<!-- Empty state (MASTER PLAN §52) -->
		<div class="rounded-lg border border-gray-200 bg-white px-4 py-12 text-center dark:border-gray-800 dark:bg-gray-800/50">
			<div class="text-sm font-medium text-gray-700 dark:text-gray-300">Henüz bölüm yok.</div>
			<div class="mt-1 text-xs text-gray-500 dark:text-gray-400">
				Örneğin "A Blok" ekleyin; ardından katlar ve daireleri toplu üretin.
			</div>
			{#if canManage}
				<div class="mt-3 flex justify-center gap-2">
					<AppButton size="sm" onclick={() => openCreate(null)}>İlk Bölümü Ekle</AppButton>
				</div>
			{/if}
		</div>
	{:else}
		<div class="rounded-lg border border-gray-200 bg-white p-2 dark:border-gray-800 dark:bg-gray-800/50">
			{#each tree as node (node.id)}
				<SectionTreeNode
					{node}
					{canManage}
					isExpanded={(id) => expanded[id] === true}
					onToggle={toggle}
					onAddChild={openCreate}
					onBulkChild={openBulk}
					onClone={openClone}
					onDelete={openDelete}
					onOpenItems={openItems}
					itemCount={countItems(node.id)}
					itemCountById={countItems}
				/>
			{/each}
		</div>
	{/if}
</div>

<!-- Bölüm ekleme modalı -->
<AppModal bind:open={createOpen} title={createParent ? `"${createParent.name}" altına bölüm ekle` : 'Kök seviyeye bölüm ekle'}>
	<form id="section-create" bind:this={createFormEl} class="flex flex-col gap-4" onsubmit={submitCreate}>
		{#if createError}
			<div class="rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
				{createError}
			</div>
		{/if}
		<div>
			<Label for="sec-name">Ad</Label>
			<Input id="sec-name" placeholder="A Blok" bind:value={createName} required />
		</div>
		<div class="grid grid-cols-2 gap-3">
			<div>
				<Label for="sec-code">Kod (opsiyonel)</Label>
				<Input id="sec-code" placeholder="A" bind:value={createCode} />
			</div>
			<div>
				<Label for="sec-type">Tip (opsiyonel)</Label>
				<Select id="sec-type" bind:value={createType}>
					<option value="">—</option>
					<option value="BLOCK">Blok</option>
					<option value="FLOOR">Kat</option>
					<option value="APARTMENT">Daire</option>
					<option value="ROOM">Oda</option>
				</Select>
			</div>
		</div>
	</form>
	{#snippet footer()}
		<AppButton color="alternative" onclick={() => (createOpen = false)}>İptal</AppButton>
		<AppButton
			disabled={busy || !createName.trim()}
			onclick={() => createFormEl?.requestSubmit()}
		>
			{busy ? 'Ekleniyor…' : 'Ekle'}
		</AppButton>
	{/snippet}
</AppModal>

<!-- Toplu oluşturma modalı -->
<AppModal bind:open={bulkOpen} title={bulkParent ? `"${bulkParent.name}" altına toplu oluştur` : 'Kök seviyeye toplu oluştur'}>
	<form id="section-bulk" bind:this={bulkFormEl} class="flex flex-col gap-4" onsubmit={submitBulk}>
		{#if bulkError}
			<div class="rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
				{bulkError}
			</div>
		{/if}
		<div class="grid grid-cols-2 gap-3">
			<div>
				<Label for="bulk-start">Başlangıç</Label>
				<Input id="bulk-start" type="number" min="0" bind:value={bulkStart} required />
			</div>
			<div>
				<Label for="bulk-count">Adet</Label>
				<Input id="bulk-count" type="number" min="1" max="200" bind:value={bulkCount} required />
			</div>
		</div>
		<div>
			<Label for="bulk-format">İsim formatı</Label>
			<Input id="bulk-format" placeholder={'Daire {n}'} bind:value={bulkNameFormat} required />
			<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
				{'{n}'} sıra numarası, {'{n:2}'} sıfır dolgulu — örnek: {bulkNameFormat.includes('{n}')
					? bulkNameFormat.replace('{n}', String(bulkStart))
					: 'Daire 1'}
			</p>
		</div>
		<div class="grid grid-cols-2 gap-3">
			<div>
				<Label for="bulk-code">Kod formatı (opsiyonel)</Label>
				<Input id="bulk-code" placeholder={'D{n:2}'} bind:value={bulkCodeFormat} />
			</div>
			<div>
				<Label for="bulk-type">Tip (opsiyonel)</Label>
				<Select id="bulk-type" bind:value={bulkType}>
					<option value="">—</option>
					<option value="BLOCK">Blok</option>
					<option value="FLOOR">Kat</option>
					<option value="APARTMENT">Daire</option>
					<option value="ROOM">Oda</option>
				</Select>
			</div>
		</div>
	</form>
	{#snippet footer()}
		<AppButton color="alternative" onclick={() => (bulkOpen = false)}>İptal</AppButton>
		<AppButton
			disabled={busy || bulkCount < 1 || !bulkNameFormat.trim()}
			onclick={() => bulkFormEl?.requestSubmit()}
		>
			{busy ? 'Oluşturuluyor…' : `${bulkCount} Bölüm Oluştur`}
		</AppButton>
	{/snippet}
</AppModal>

<!-- Çoğaltma modalı -->
<AppModal bind:open={cloneOpen} title={`"${cloneNode?.name ?? ''}" çoğalt`}>
	<form id="section-clone" bind:this={cloneFormEl} class="flex flex-col gap-4" onsubmit={submitClone}>
		{#if cloneError}
			<div class="rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
				{cloneError}
			</div>
		{/if}
		<p class="text-sm text-gray-500 dark:text-gray-400">
			Kaynağın <strong>tüm alt ağacı</strong> (alt bölümleriyle birlikte) kopyalanır.
		</p>
		<div>
			<Label for="clone-count">Kopya adedi</Label>
			<Input id="clone-count" type="number" min="1" max="50" bind:value={cloneCount} required />
		</div>
		<div>
			<Label for="clone-format">İsim formatı (opsiyonel)</Label>
			<Input
				id="clone-format"
				placeholder={'Kat {n:2} — boş bırak: otomatik'}
				bind:value={cloneNameFormat}
			/>
			{#if clonePreview}
				<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">Önizleme: {clonePreview}</p>
			{/if}
		</div>
	</form>
	{#snippet footer()}
		<AppButton color="alternative" onclick={() => (cloneOpen = false)}>İptal</AppButton>
		<AppButton
			disabled={busy || cloneCount < 1}
			onclick={() => cloneFormEl?.requestSubmit()}
		>
			{busy ? 'Çoğaltılıyor…' : `${cloneCount}× Çoğalt`}
		</AppButton>
	{/snippet}
</AppModal>

<!-- Silme onayı -->
<AppModal bind:open={deleteOpen} title="Bölümü sil">
	<div class="space-y-3">
		{#if deleteError}
			<div class="rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
				{deleteError}
			</div>
		{/if}
		<p class="text-sm text-gray-700 dark:text-gray-300">
			<strong>"{deleteNode?.name}"</strong> bölümü silinecek. Bu işlem geri alınamaz (kayıt geçmişte
			arşivlenir).
		</p>
		<p class="text-xs text-gray-500 dark:text-gray-400">
			Alt bölümü varsa silme reddedilir; önce alt bölümleri silin.
		</p>
	</div>
	{#snippet footer()}
		<AppButton color="alternative" onclick={() => (deleteOpen = false)}>Vazgeç</AppButton>
		<AppButton color="red" disabled={busy} onclick={confirmDelete}>
			{busy ? 'Siliniyor…' : 'Sil'}
		</AppButton>
	{/snippet}
</AppModal>

<!-- Bölüm iş kalemleri drawer -->
<WorkItemsDrawer
	bind:open={drawerOpen}
	section={selectedSection}
	{projectId}
	{canManage}
	isAdmin={session.isAdmin}
/>

<!-- Yaprak bölümlere toplu iş kalemi modalı -->
<AppModal bind:open={bulkItemsOpen} title="Yaprak bölümlere iş kalemi ekle">
	<form id="bulk-items-form" bind:this={bulkItemsFormEl} class="flex flex-col gap-4" onsubmit={submitBulkItems}>
		{#if bulkItemsError}
			<div class="rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
				{bulkItemsError}
			</div>
		{/if}
		<p class="text-sm text-gray-500 dark:text-gray-400">
			Seçilen bölümün altındaki tüm <strong>yaprak</strong> bölümlere (alt bölümü olmayanlara) bu
			tipte birer iş kalemi eklenir. Mevcut olanlar atlanır.
		</p>
		<div>
			<Label for="bi-parent">Üst bölüm</Label>
			<Select id="bi-parent" bind:value={bulkItemsParent} required>
				<option value="">Seçin…</option>
				{#each flatten(tree ?? []) as { node, depth } (node.id)}
					<option value={node.id}>{' '.repeat(depth * 2)}{node.name}</option>
				{/each}
			</Select>
		</div>
		<div>
			<Label for="bi-type">İş kalemi tipi</Label>
			<Select id="bi-type" bind:value={bulkItemsTypeId} required>
				<option value="">Seçin…</option>
				{#each types as t (t.id)}
					<option value={t.id}>{t.name} ({t.code})</option>
				{/each}
			</Select>
			{#if types.length === 0}
				<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
					Tip yoksa önce herhangi bir bölümün iş kalemi panelinden yeni tip tanımlayın.
				</p>
			{/if}
		</div>
	</form>
	{#snippet footer()}
		<AppButton color="alternative" onclick={() => (bulkItemsOpen = false)}>İptal</AppButton>
		<AppButton
			disabled={bulkItemsBusy || !bulkItemsParent || !bulkItemsTypeId}
			onclick={() => bulkItemsFormEl?.requestSubmit()}
		>
			{bulkItemsBusy ? 'Ekleniyor…' : 'Ekle'}
		</AppButton>
	{/snippet}
</AppModal>
