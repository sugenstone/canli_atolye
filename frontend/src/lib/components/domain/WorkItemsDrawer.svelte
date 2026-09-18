<script lang="ts">
	/**
	 * Bölüm iş kalemleri drawer'ı (MASTER PLAN §28 hücre detay panelinin temeli):
	 * iş kalemi listesi + ekleme + silme + dinamik özellik görünümü/düzenlemesi.
	 * Kendi verisini yönetir; kapanınca state temizlenir.
	 */
	import { Drawer, Input, Label, Select, Checkbox } from 'flowbite-svelte';
	import StatusBadge from '$lib/components/ui/StatusBadge.svelte';
	import AppButton from '$lib/components/ui/AppButton.svelte';
	import {
		propertyDefinitionService,
		workItemService,
		workItemTypeService,
		parseOptions
	} from '$lib/services/api/workItems';
	import {
		processExecutionService,
		processGroupService
	} from '$lib/services/api/processes';
	import {
		attachmentService,
		blockReasonService,
		blockService,
		directoryService,
		noteService
	} from '$lib/services/api/operations';
	import { ApiError } from '$lib/services/api/client';
	import { authService } from '$lib/services/api/auth';
	import { PRIORITY, statusToken } from '$lib/config/status';
	import type {
		AttachmentDto,
		BlockReasonDto,
		ExecutionRow,
		Priority,
		ProcessGroupDto,
		PropertyDefinitionDto,
		SectionTreeNode,
		WorkItemDetailDto,
		WorkItemDto,
		WorkItemTypeDto
	} from '$lib/types';

	let {
		open = $bindable(false),
		section,
		projectId,
		canManage = false,
		isAdmin = false
	}: {
		open?: boolean;
		section: SectionTreeNode | null;
		projectId: string;
		canManage?: boolean;
		isAdmin?: boolean;
	} = $props();

	let items = $state<WorkItemDto[] | null>(null);
	let types = $state<WorkItemTypeDto[]>([]);
	let error = $state<string | null>(null);
	let busy = $state(false);

	// Seçili item detayı (özellik + süreç paneli)
	let expandedItemId = $state<string | null>(null);
	let detail = $state<WorkItemDetailDto | null>(null);
	let draftValues = $state<Record<string, string>>({});
	// Süreçler
	let executions = $state<ExecutionRow[] | null>(null);
	let groups = $state<ProcessGroupDto[]>([]);
	let assignGroupId = $state('');
	let processBusy = $state(false);

	// Faz 5: bloke / atama / not / dosya
	let reasons = $state<BlockReasonDto[]>([]);
	let users = $state<{ id: string; full_name: string; role: string }[]>([]);
	let teams = $state<{ id: string; name: string }[]>([]);
	let reportExecId = $state<string | null>(null);
	let reportReason = $state('');
	let reportDesc = $state('');
	let assignExecId = $state<string | null>(null);
	let assignUser = $state('');
	let assignTeam = $state('');
	let noteExecId = $state<string | null>(null);
	let noteText = $state('');
	let attachments = $state<AttachmentDto[]>([]);
	let uploadBusy = $state(false);
	let fileInputEl = $state<HTMLInputElement | undefined>(undefined);

	// Ekleme formu
	let showAdd = $state(false);
	let newTypeId = $state('');
	let newName = $state('');
	let newPriority = $state<Priority>('NORMAL');

	// ADMIN: yeni tip / yeni özellik tanımı
	let showNewType = $state(false);
	let typeFormEl = $state<HTMLFormElement | undefined>();
	let newTypeName = $state('');
	let newTypeCode = $state('');
	let showNewProp = $state(false);
	let propFormEl = $state<HTMLFormElement | undefined>();
	let propName = $state('');
	let propType = $state<'TEXT' | 'NUMBER' | 'BOOLEAN' | 'SELECT'>('TEXT');
	let propUnit = $state('');
	let propOptions = $state('');
	let definitions = $state<PropertyDefinitionDto[]>([]);

	// Section değişince yükle
	$effect(() => {
		if (open && section) {
			items = null;
			error = null;
			expandedItemId = null;
			detail = null;
			showAdd = false;
			void loadAll();
		}
	});

	async function loadAll() {
		if (!section) return;
		try {
			const [itemList, typeList] = await Promise.all([
				workItemService.listBySection(projectId, section.id),
				workItemTypeService.list()
			]);
			items = itemList;
			types = typeList;
			if (isAdmin) {
				definitions = await propertyDefinitionService.list();
			}
			if (canManage) {
				groups = await processGroupService.list().catch(() => []);
			}
			reasons = await blockReasonService.list().catch(() => []);
			if (canManage) {
				const me = await authService.me().catch(() => null);
				const [workspaceUsers, workspaceTeams] = await Promise.all([
					me ? directoryService.users(me.workspace.id).catch(() => []) : Promise.resolve([]),
					directoryService.teams().catch(() => [])
				]);
				users = workspaceUsers;
				teams = workspaceTeams;
			}
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'İş kalemleri yüklenemedi.';
		}
	}

	async function toggleDetail(itemId: string) {
		if (expandedItemId === itemId) {
			expandedItemId = null;
			detail = null;
			return;
		}
		expandedItemId = itemId;
		await loadDetail(itemId);
	}

	/** Detayı ve süreçleri çek; değerleri form taslağına doldur. */
	async function loadDetail(itemId: string) {
		detail = null;
		draftValues = {};
		executions = null;
		try {
			[detail, executions, attachments] = await Promise.all([
				workItemService.get(itemId),
				processExecutionService
					.listByWorkItem(itemId)
					.then((r) => r.executions)
					.catch(() => []),
				attachmentService.list('WORK_ITEM', itemId).catch(() => [])
			]);
			for (const row of detail.properties) {
				if (row.value_number != null) draftValues[row.definition_id] = String(row.value_number);
				else if (row.value_boolean != null) draftValues[row.definition_id] = row.value_boolean ? 'true' : 'false';
				else if (row.value_text != null) draftValues[row.definition_id] = row.value_text;
			}
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Detay yüklenemedi.';
		}
	}

	async function addItem(e: SubmitEvent) {
		e.preventDefault();
		if (busy || !section || !newTypeId) return;
		busy = true;
		error = null;
		try {
			await workItemService.create(projectId, {
				section_id: section.id,
				work_item_type_id: newTypeId,
				name: newName.trim() || undefined,
				priority: newPriority
			});
			newName = '';
			newPriority = 'NORMAL';
			showAdd = false;
			await loadAll();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'İş kalemi eklenemedi.';
		} finally {
			busy = false;
		}
	}

	async function assignProcessGroup() {
		if (processBusy || !expandedItemId || !assignGroupId) return;
		processBusy = true;
		error = null;
		try {
			await processExecutionService.assignGroup(expandedItemId, assignGroupId);
			assignGroupId = '';
			await loadDetail(expandedItemId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Süreç grubu atanamadı.';
		} finally {
			processBusy = false;
		}
	}

	async function act(executionId: string, action: 'start' | 'pause' | 'resume' | 'complete') {
		if (processBusy) return;
		processBusy = true;
		error = null;
		try {
			await processExecutionService.act(executionId, action);
			if (expandedItemId) await loadDetail(expandedItemId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'İşlem başarısız.';
		} finally {
			processBusy = false;
		}
	}

	function openReport(execId: string) {
		reportExecId = execId;
		reportReason = '';
		reportDesc = '';
	}

	async function submitReport() {
		if (processBusy || !reportExecId || !reportReason) return;
		processBusy = true;
		error = null;
		try {
			await blockService.report(reportExecId, reportReason, reportDesc.trim() || undefined);
			reportExecId = null;
			if (expandedItemId) await loadDetail(expandedItemId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Bloke başarısız.';
		} finally {
			processBusy = false;
		}
	}

	async function resolve(execId: string) {
		if (processBusy) return;
		processBusy = true;
		error = null;
		try {
			await blockService.resolve(execId);
			if (expandedItemId) await loadDetail(expandedItemId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Bloke kaldırılamadı.';
		} finally {
			processBusy = false;
		}
	}

	function openAssign(execId: string) {
		assignExecId = execId;
		assignUser = '';
		assignTeam = '';
	}

	async function submitAssign() {
		if (processBusy || !assignExecId) return;
		if (!assignUser && !assignTeam) {
			assignExecId = null;
			return;
		}
		processBusy = true;
		error = null;
		try {
			await processExecutionService.assign(assignExecId, {
				...(assignUser ? { user_id: assignUser } : {}),
				...(assignTeam ? { team_id: assignTeam } : {})
			});
			assignExecId = null;
			if (expandedItemId) await loadDetail(expandedItemId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Atama başarısız.';
		} finally {
			processBusy = false;
		}
	}

	async function submitNote() {
		if (processBusy || !noteExecId || !noteText.trim()) return;
		processBusy = true;
		error = null;
		try {
			await noteService.add(noteExecId, noteText.trim());
			noteExecId = null;
			noteText = '';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Not eklenemedi.';
		} finally {
			processBusy = false;
		}
	}

	async function uploadFile(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file || uploadBusy || !expandedItemId) return;
		uploadBusy = true;
		error = null;
		try {
			await attachmentService.upload('WORK_ITEM', expandedItemId, file);
			await loadDetail(expandedItemId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Yükleme başarısız.';
		} finally {
			uploadBusy = false;
			input.value = '';
		}
	}

	async function removeAttachment(id: string) {
		if (uploadBusy) return;
		uploadBusy = true;
		error = null;
		try {
			await attachmentService.remove(id);
			if (expandedItemId) await loadDetail(expandedItemId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Silme başarısız.';
		} finally {
			uploadBusy = false;
		}
	}

	async function removeItem(itemId: string) {
		if (busy) return;
		busy = true;
		error = null;
		try {
			await workItemService.remove(itemId);
			if (expandedItemId === itemId) {
				expandedItemId = null;
				detail = null;
			}
			await loadAll();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Silme başarısız.';
		} finally {
			busy = false;
		}
	}

	async function saveValues() {
		if (busy || !detail) return;
		busy = true;
		error = null;
		try {
			const savedItemId = detail.item.id;
			const payload = detail.properties.map((row) => {
				// bind:value number inputlarda number dönebilir → normalize
				const raw = draftValues[row.definition_id];
				const text = raw === undefined || raw === null ? '' : String(raw);
				let value: string | number | boolean | null = null;
				if (text !== '') {
					if (row.data_type === 'NUMBER' || row.data_type === 'DECIMAL') {
						value = Number(text.replace(',', '.'));
					} else if (row.data_type === 'BOOLEAN') {
						value = text === 'true';
					} else {
						value = text;
					}
				}
				return { property_definition_id: row.definition_id, value };
			});
			await workItemService.setValues(savedItemId, payload);
			await loadDetail(savedItemId); // değerleri taze göster
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Özellikler kaydedilemedi.';
		} finally {
			busy = false;
		}
	}

	async function createType(e: SubmitEvent) {
		e.preventDefault();
		if (busy) return;
		busy = true;
		error = null;
		try {
			const created = await workItemTypeService.create({
				name: newTypeName.trim(),
				code: newTypeCode.trim()
			});
			showNewType = false;
			newTypeName = '';
			newTypeCode = '';
			await loadAll();
			newTypeId = created.id;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Tip oluşturulamadı.';
		} finally {
			busy = false;
		}
	}

	async function createProperty(e: SubmitEvent) {
		e.preventDefault();
		if (busy) return;
		busy = true;
		error = null;
		try {
			await propertyDefinitionService.create({
				name: propName.trim(),
				data_type: propType,
				unit: propUnit.trim() || undefined,
				options:
					propType === 'SELECT' && propOptions.trim()
						? propOptions.split(',').map((s) => s.trim()).filter(Boolean)
						: undefined
			});
			showNewProp = false;
			propName = '';
			propUnit = '';
			propOptions = '';
			definitions = await propertyDefinitionService.list();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Özellik tanımı oluşturulamadı.';
		} finally {
			busy = false;
		}
	}

	function typeName(typeId: string): string {
		return types.find((t) => t.id === typeId)?.name ?? '';
	}
</script>

<Drawer bind:open placement="right" class="w-full max-w-md p-0">
	<div class="flex h-full flex-col">
		<header class="flex items-start justify-between gap-2 border-b border-gray-200 px-4 py-3 dark:border-gray-700">
			<div class="min-w-0">
				<div class="text-[11px] font-medium uppercase tracking-wide text-gray-400">Bölüm iş kalemleri</div>
				<h2 class="truncate text-base font-semibold text-gray-900 dark:text-white">
					{section?.name ?? ''}
				</h2>
			</div>
			{#if canManage}
				<AppButton size="sm" onclick={() => (showAdd = !showAdd)}>+ Ekle</AppButton>
			{/if}
		</header>

		<div class="flex-1 overflow-y-auto px-4 py-3">
			{#if error}
				<div class="mb-3 rounded-lg border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
					{error}
				</div>
			{/if}

			<!-- Ekleme formu -->
			{#if showAdd && canManage}
				<form class="mb-4 flex flex-col gap-3 rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-800/60" onsubmit={addItem}>
					{#if types.length === 0}
						<p class="text-xs text-gray-500 dark:text-gray-400">
							Önce bir iş kalemi tipi gerekli (örn. "Mutfak Tezgahı").
						</p>
						{#if isAdmin}
							<AppButton size="sm" color="alternative" onclick={() => (showNewType = !showNewType)}>
								Yeni Tip Tanımla
							</AppButton>
						{/if}
					{:else}
						<div>
							<Label for="wi-type">Tip</Label>
							<Select id="wi-type" bind:value={newTypeId} required>
								<option value="">Seçin…</option>
								{#each types as t (t.id)}
									<option value={t.id}>{t.name} ({t.code})</option>
								{/each}
							</Select>
						</div>
						<div class="grid grid-cols-2 gap-2">
							<div>
								<Label for="wi-name">Ad (opsiyonel)</Label>
								<Input id="wi-name" placeholder="Tip adı kullanılır" bind:value={newName} />
							</div>
							<div>
								<Label for="wi-priority">Öncelik</Label>
								<Select id="wi-priority" bind:value={newPriority}>
									<option value="LOW">Düşük</option>
									<option value="NORMAL">Normal</option>
									<option value="HIGH">Yüksek</option>
									<option value="URGENT">Acil</option>
								</Select>
							</div>
						</div>
						<div class="flex justify-end gap-2">
							{#if isAdmin}
								<AppButton size="sm" color="alternative" onclick={() => (showNewType = !showNewType)}>Yeni Tip</AppButton>
							{/if}
							<AppButton size="sm" type="submit" disabled={busy || !newTypeId}>
								{busy ? 'Ekleniyor…' : 'Ekle'}
							</AppButton>
						</div>
					{/if}
				</form>

				<!-- Yeni tip formu (ADMIN) -->
				{#if showNewType && isAdmin}
					<form bind:this={typeFormEl} class="mb-4 flex flex-col gap-3 rounded-lg border border-blue-200 bg-blue-50/50 p-3 dark:border-blue-800 dark:bg-blue-900/20" onsubmit={createType}>
						<div class="text-xs font-semibold text-blue-700 dark:text-blue-300">Yeni İş Kalemi Tipi</div>
						<div class="grid grid-cols-2 gap-2">
							<div>
								<Label for="nt-name">Ad</Label>
								<Input id="nt-name" placeholder="Mutfak Tezgahı" bind:value={newTypeName} required />
							</div>
							<div>
								<Label for="nt-code">Kod</Label>
								<Input id="nt-code" placeholder="MT" bind:value={newTypeCode} required />
							</div>
						</div>
						<div class="flex justify-end">
							<AppButton size="sm" type="submit" disabled={busy}>Oluştur</AppButton>
						</div>
					</form>
				{/if}
			{/if}

			<!-- Liste -->
			{#if items === null}
				<div class="space-y-2">
					{#each Array(3) as _}
						<div class="h-14 animate-pulse rounded-lg bg-gray-200 dark:bg-gray-700"></div>
					{/each}
				</div>
			{:else if items.length === 0}
				<div class="py-8 text-center">
					<div class="text-sm text-gray-500 dark:text-gray-400">Bu bölümde iş kalemi yok.</div>
					{#if canManage}
						<div class="mt-1 text-xs text-gray-400">"+ Ekle" ile başlayın.</div>
					{/if}
				</div>
			{:else}
				<ul class="space-y-2">
					{#each items as item (item.id)}
						<li class="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
							<div class="flex items-center gap-2 px-3 py-2.5">
								<button type="button" class="flex min-w-0 flex-1 items-center gap-2 text-left" onclick={() => toggleDetail(item.id)}>
									<svg class="h-3.5 w-3.5 shrink-0 text-gray-400 transition-transform {expandedItemId === item.id ? 'rotate-90' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
										<path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
									</svg>
									<span class="truncate text-sm font-medium text-gray-900 dark:text-white">{item.name}</span>
									<span class="shrink-0 rounded bg-gray-100 px-1.5 py-0.5 text-[10px] text-gray-500 dark:bg-gray-700 dark:text-gray-300">
										{typeName(item.work_item_type_id)}
									</span>
								</button>
								<StatusBadge status={item.status} kind="process" size="xs" />
								{#if item.priority !== 'NORMAL'}
									<span class="hidden shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium sm:inline {statusToken(PRIORITY, item.priority).badgeClass}">
										{statusToken(PRIORITY, item.priority).label}
									</span>
								{/if}
								{#if canManage}
									<button type="button" class="rounded p-1 text-gray-400 hover:bg-red-50 hover:text-red-600 dark:hover:bg-red-900/30" title="Sil" aria-label="{item.name} iş kalemini sil" onclick={() => removeItem(item.id)}>
										<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
											<path stroke-linecap="round" stroke-linejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
										</svg>
									</button>
								{/if}
							</div>

							<!-- Özellik paneli -->
							{#if expandedItemId === item.id}
								<div class="border-t border-gray-100 bg-gray-50 px-3 py-3 dark:border-gray-700 dark:bg-gray-800/40">
									{#if detail === null}
										<div class="h-16 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
									{:else}
										<!-- Süreçler (state machine görünümü) -->
										<div class="mb-4">
											<div class="mb-2 flex items-center justify-between">
												<span class="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">Süreçler</span>
												{#if canManage && executions !== null && executions.length === 0 && groups.length > 0}
													<div class="flex items-center gap-1.5">
														<select class="rounded-md border border-gray-300 bg-white p-1 text-xs dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={assignGroupId}>
															<option value="">Grup seçin…</option>
															{#each groups as g (g.id)}
																<option value={g.id}>{g.name}</option>
															{/each}
														</select>
														<AppButton size="xs" disabled={processBusy || !assignGroupId} onclick={assignProcessGroup}>Ata</AppButton>
													</div>
												{/if}
											</div>
											{#if executions === null}
												<div class="h-12 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
											{:else if executions.length === 0}
												<div class="rounded-md border border-dashed border-gray-300 px-3 py-2 text-xs text-gray-400 dark:border-gray-600">
													Henüz süreç atanmadı.{#if canManage && groups.length > 0} Yukarıdan bir süreç grubu atayın.{/if}
												</div>
											{:else}
												<ol class="space-y-1.5">
													{#each executions as ex (ex.id)}
														<li class="flex flex-wrap items-center gap-2 rounded-md border border-gray-200 bg-white px-2.5 py-2 dark:border-gray-600 dark:bg-gray-700/50">
															<span class="text-xs font-semibold text-gray-400">{ex.sort_order}</span>
															<span class="text-sm font-medium text-gray-900 dark:text-white">{ex.template_name}</span>
															<StatusBadge status={ex.status} kind="process" size="xs" />
															{#if ex.started_at}
																<span class="text-[10px] text-gray-400">{new Date(ex.started_at).toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit' })}</span>
															{/if}
															{#if canManage}
																<span class="ms-auto flex flex-wrap gap-1">
																	{#if ex.status === 'READY'}
																		<AppButton size="xs" disabled={processBusy} onclick={() => act(ex.id, 'start')}>Başlat</AppButton>
																	{:else if ex.status === 'IN_PROGRESS'}
																		<AppButton size="xs" color="alternative" disabled={processBusy} onclick={() => act(ex.id, 'pause')}>Duraklat</AppButton>
																		<AppButton size="xs" disabled={processBusy} onclick={() => act(ex.id, 'complete')}>Bitir</AppButton>
																	{:else if ex.status === 'PAUSED'}
																		<AppButton size="xs" disabled={processBusy} onclick={() => act(ex.id, 'resume')}>Devam</AppButton>
																		<AppButton size="xs" disabled={processBusy} onclick={() => act(ex.id, 'complete')}>Bitir</AppButton>
																	{/if}
																	{#if ex.status === 'READY' || ex.status === 'IN_PROGRESS'}
																		<AppButton size="xs" color="red" disabled={processBusy} onclick={() => openReport(ex.id)}>Sorun</AppButton>
																	{/if}
																	{#if ex.status === 'BLOCKED'}
																		<AppButton size="xs" disabled={processBusy} onclick={() => resolve(ex.id)}>Blokeyi Kaldır</AppButton>
																	{/if}
																	<AppButton size="xs" color="alternative" disabled={processBusy} onclick={() => openAssign(ex.id)}>Ata</AppButton>
																	<button
																		type="button"
																		class="rounded p-1 text-gray-400 hover:bg-blue-50 hover:text-blue-600 dark:hover:bg-blue-900/30"
																		title="Not ekle"
																		onclick={() => { noteExecId = noteExecId === ex.id ? null : ex.id; noteText = ''; }}
																	>&#9998;</button>
																</span>
															{/if}
															{#if ex.assigned_user_id || ex.assigned_team_id}
																<span class="w-full text-[10px] text-gray-400">
																	atanan: {users.find((u) => u.id === ex.assigned_user_id)?.full_name ?? teams.find((t) => t.id === ex.assigned_team_id)?.name ?? '—'}
																</span>
															{/if}
															{#if noteExecId === ex.id}
																<div class="flex w-full gap-1.5">
																	<input
																		class="flex-1 rounded-md border border-gray-300 bg-white p-1.5 text-xs dark:border-gray-600 dark:bg-gray-700 dark:text-white"
																		placeholder="Not yazın…"
																		bind:value={noteText}
																		onkeydown={(e) => e.key === 'Enter' && submitNote()}
																	/>
																	<AppButton size="xs" disabled={processBusy || !noteText.trim()} onclick={submitNote}>Gönder</AppButton>
																</div>
															{/if}
															{#if reportExecId === ex.id}
																<div class="w-full space-y-2 rounded-md border border-red-200 bg-red-50/60 p-2 dark:border-red-800 dark:bg-red-950/20">
																	<div class="text-xs font-semibold text-red-700 dark:text-red-300">Sorun Bildir</div>
																	<select class="w-full rounded-md border border-gray-300 bg-white p-1.5 text-xs dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={reportReason}>
																		<option value="">Neden seçin…</option>
																		{#each reasons as r (r.id)}
																			<option value={r.id}>{r.name}</option>
																		{/each}
																	</select>
																	<input
																		class="w-full rounded-md border border-gray-300 bg-white p-1.5 text-xs dark:border-gray-600 dark:bg-gray-700 dark:text-white"
																		placeholder="Açıklama (opsiyonel)"
																		bind:value={reportDesc}
																	/>
																	<div class="flex justify-end gap-1.5">
																		<AppButton size="xs" color="alternative" onclick={() => (reportExecId = null)}>Vazgeç</AppButton>
																		<AppButton size="xs" color="red" disabled={processBusy || !reportReason} onclick={submitReport}>Bildir</AppButton>
																	</div>
																</div>
															{/if}
															{#if assignExecId === ex.id}
																<div class="w-full space-y-2 rounded-md border border-blue-200 bg-blue-50/60 p-2 dark:border-blue-800 dark:bg-blue-950/20">
																	<div class="text-xs font-semibold text-blue-700 dark:text-blue-300">Ata</div>
																	<div class="grid grid-cols-2 gap-1.5">
																		<select class="rounded-md border border-gray-300 bg-white p-1.5 text-xs dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={assignUser}>
																			<option value="">Kullanıcı —</option>
																			{#each users as u (u.id)}
																				<option value={u.id}>{u.full_name}</option>
																			{/each}
																		</select>
																		<select class="rounded-md border border-gray-300 bg-white p-1.5 text-xs dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={assignTeam}>
																			<option value="">Takım —</option>
																			{#each teams as t (t.id)}
																				<option value={t.id}>{t.name}</option>
																			{/each}
																		</select>
																	</div>
																	<div class="flex justify-end gap-1.5">
																		<AppButton size="xs" color="alternative" onclick={() => (assignExecId = null)}>Vazgeç</AppButton>
																		<AppButton size="xs" disabled={processBusy || (!assignUser && !assignTeam)} onclick={submitAssign}>Kaydet</AppButton>
																	</div>
																</div>
															{/if}
														</li>
													{/each}
												</ol>
											{/if}
										</div>
									{/if}
									{#if detail === null}
										<div class="h-16 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
									{:else if detail.properties.length === 0}
										<div class="text-center text-xs text-gray-500 dark:text-gray-400">
											Henüz özellik girilmedi.
											{#if isAdmin && definitions.length === 0}
												<br />Yönetici olarak yeni özellik tanımı oluşturabilirsiniz.
											{/if}
										</div>
									{:else}
										<div class="grid grid-cols-1 gap-2.5">
											{#each detail.properties as row (row.definition_id)}
												<div class="flex items-center gap-2">
													<label class="w-28 shrink-0 truncate text-xs font-medium text-gray-600 dark:text-gray-300" for="prop-{row.definition_id}" title="{row.definition_name}">
														{row.definition_name}{row.unit ? ` (${row.unit})` : ''}
													</label>
													{#if row.data_type === 'SELECT'}
														<select id="prop-{row.definition_id}" class="w-full rounded-lg border border-gray-300 bg-white p-1.5 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={draftValues[row.definition_id]}>
															<option value="">—</option>
															{#each parseOptions(row.options) as opt}
																<option value={opt}>{opt}</option>
															{/each}
														</select>
													{:else if row.data_type === 'BOOLEAN'}
														<select id="prop-{row.definition_id}" class="w-full rounded-lg border border-gray-300 bg-white p-1.5 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={draftValues[row.definition_id]}>
															<option value="">—</option>
															<option value="true">Evet</option>
															<option value="false">Hayır</option>
														</select>
													{:else}
														<input
															id="prop-{row.definition_id}"
															type={row.data_type === 'NUMBER' || row.data_type === 'DECIMAL' ? 'number' : 'text'}
															step="any"
															class="w-full rounded-lg border border-gray-300 bg-white p-1.5 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white"
															bind:value={draftValues[row.definition_id]}
														/>
													{/if}
												</div>
											{/each}
										</div>
										{#if canManage}
											<div class="mt-3 flex justify-end">
												<AppButton size="sm" disabled={busy} onclick={saveValues}>
													{busy ? 'Kaydediliyor…' : 'Özellikleri Kaydet'}
												</AppButton>
											</div>
										{/if}

										<!-- Dosya ekleri -->
										<div class="mt-4 border-t border-gray-100 pt-3 dark:border-gray-700">
											<div class="mb-2 flex items-center justify-between">
												<span class="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">Dosyalar</span>
												{#if canManage}
													<label class="cursor-pointer rounded-md bg-blue-600 px-2 py-1 text-[11px] font-medium text-white hover:bg-blue-700">
																						{uploadBusy ? 'Yükleniyor…' : '+ Dosya'}
																						<input type="file" class="hidden" accept=".jpg,.jpeg,.png,.webp,.pdf,.xlsx,.dwg,.dxf" onchange={uploadFile} bind:this={fileInputEl} />
																					</label>
																				{/if}
																			</div>
																			{#if attachments.length === 0}
																				<div class="text-xs text-gray-400 dark:text-gray-500">Dosya eklenmemiş.</div>
																			{:else}
																				<ul class="space-y-1">
																					{#each attachments as att (att.id)}
																						<li class="flex items-center gap-2 text-xs">
																							<a href={attachmentService.downloadUrl(att.id)} class="min-w-0 flex-1 truncate text-blue-600 hover:underline dark:text-blue-400" title={att.file_name}>
																								{att.file_name}
																							</a>
																							<span class="shrink-0 text-gray-400">{Math.ceil(att.size / 1024)} KB</span>
																							{#if canManage}
																								<button type="button" class="shrink-0 rounded p-0.5 text-gray-400 hover:text-red-600" title="Sil" onclick={() => removeAttachment(att.id)}>&#10005;</button>
																							{/if}
																						</li>
																					{/each}
																				</ul>
																			{/if}
																		</div>
									{/if}
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}

			<!-- Yeni özellik tanımı (ADMIN) -->
			{#if isAdmin}
				<div class="mt-4 border-t border-gray-200 pt-3 dark:border-gray-700">
					{#if showNewProp}
						<form bind:this={propFormEl} class="flex flex-col gap-3 rounded-lg border border-blue-200 bg-blue-50/50 p-3 dark:border-blue-800 dark:bg-blue-900/20" onsubmit={createProperty}>
							<div class="text-xs font-semibold text-blue-700 dark:text-blue-300">Yeni Özellik Tanımı (workspace)</div>
							<div class="grid grid-cols-2 gap-2">
								<div>
									<Label for="np-name">Ad</Label>
									<Input id="np-name" placeholder="Metraj" bind:value={propName} required />
								</div>
								<div>
									<Label for="np-type">Veri tipi</Label>
									<Select id="np-type" bind:value={propType}>
										<option value="TEXT">Metin</option>
										<option value="NUMBER">Sayı</option>
										<option value="BOOLEAN">Evet/Hayır</option>
										<option value="SELECT">Seçim listesi</option>
									</Select>
								</div>
							</div>
							<div class="grid grid-cols-2 gap-2">
								<div>
									<Label for="np-unit">Birim (opsiyonel)</Label>
									<Input id="np-unit" placeholder="m" bind:value={propUnit} />
								</div>
								{#if propType === 'SELECT'}
									<div>
										<Label for="np-options">Seçenekler (virgülle)</Label>
										<Input id="np-options" placeholder="Lamar, Arma, Quartz" bind:value={propOptions} />
									</div>
								{/if}
							</div>
							<div class="flex justify-end gap-2">
								<AppButton size="sm" color="alternative" onclick={() => (showNewProp = false)}>İptal</AppButton>
								<AppButton size="sm" type="submit" disabled={busy}>Oluştur</AppButton>
							</div>
						</form>
					{:else}
						<button type="button" class="text-xs font-medium text-blue-600 hover:underline dark:text-blue-400" onclick={() => (showNewProp = true)}>
							+ Yeni özellik tanımı
						</button>
					{/if}
					{#if definitions.length > 0}
						<p class="mt-1.5 text-[11px] text-gray-400">
							Tanımlı: {definitions.map((d) => d.name).join(', ')}
						</p>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</Drawer>
