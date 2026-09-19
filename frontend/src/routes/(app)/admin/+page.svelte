<script lang="ts">
	/**
	 * Yönetim: süreç şablonları ve süreç grupları (MASTER PLAN §47 /admin).
	 * Grup kurulumunda bağımlılıklar adım indeksleriyle verilir; "Lineer yap"
	 * hızlı aksiyonu her adımı öncekine bağlar (MASTER PLAN §11: ilk sürümde
	 * çoğu süreç lineer, model çoklu bağımlılığı destekler).
	 */
	import { onMount } from 'svelte';
	import { Input, Label, Select } from 'flowbite-svelte';
	import AppButton from '$lib/components/ui/AppButton.svelte';
	import {
		processGroupService,
		processTemplateService
	} from '$lib/services/api/processes';
	import { ApiError } from '$lib/services/api/client';
	import { session } from '$lib/stores/session.svelte';
	import { authService } from '$lib/services/api/auth';
	import {
		userService,
		teamAdminService,
		ROLE_LABELS,
		type UserDto,
		type TeamDto
	} from '$lib/services/api/users';
	import { auditLogs, type AuditLogDto } from '$lib/services/api/insights';
	import type { ProcessGroupDto, ProcessTemplateDto } from '$lib/types';

	let templates = $state<ProcessTemplateDto[]>([]);
	let groups = $state<ProcessGroupDto[]>([]);
	let error = $state<string | null>(null);
	let busy = $state(false);

	// Kullanıcı yönetimi
	let users = $state<UserDto[]>([]);
	let workspaceId = $state('');
	let userName = $state('');
	let userEmail = $state('');
	let userPassword = $state('');
	let userRole = $state<'WORKER' | 'TEAM_LEADER' | 'PROJECT_MANAGER' | 'VIEWER' | 'ADMIN'>('WORKER');
	let userBusy = $state(false);

	// Takım yönetimi
	let teams = $state<TeamDto[]>([]);
	let teamName = $state('');
	let teamBusy = $state(false);
	let expandedTeamId = $state<string | null>(null);
	let audit = $state<AuditLogDto[]>([]);
	let teamMembers = $state<Record<string, { user_id: string; role: string }[]>>({});
	let memberUserId = $state('');

	// Şablon formu
	let tplName = $state('');
	let tplCode = $state('');

	// Grup formu
	let grpName = $state('');
	let grpSteps = $state<{ template_id: string; deps: number[] }[]>([]);
	let grpSelected = $state('');

	const isAdmin = $derived(session.isAdmin);

	async function load() {
		error = null;
		try {
			const me = await authService.me();
			workspaceId = me.workspace.id;
			const [templates, groups, users, teams, auditRows] = await Promise.all([
				processTemplateService.list(),
				processGroupService.list(),
				userService.list(workspaceId).catch(() => []),
				teamAdminService.list().catch(() => []),
				auditLogs().catch(() => [])
			]);
			audit = auditRows;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Yüklenemedi.';
		}
	}

	async function createUser(e: SubmitEvent) {
		e.preventDefault();
		if (userBusy || !userName.trim() || !userEmail.trim() || userPassword.length < 8) return;
		userBusy = true;
		error = null;
		try {
			await userService.create(workspaceId, {
				full_name: userName.trim(),
				email: userEmail.trim(),
				password: userPassword,
				role: userRole
			});
			userName = '';
			userEmail = '';
			userPassword = '';
			userRole = 'WORKER';
			await load();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Kullanıcı eklenemedi.';
		} finally {
			userBusy = false;
		}
	}

	async function changeRole(user: UserDto, role: UserDto['role']) {
		error = null;
		try {
			await userService.update(user.id, { role });
			await load();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Rol değiştirilemedi.';
		}
	}

	async function toggleActive(user: UserDto) {
		error = null;
		try {
			if (user.active) {
				await userService.deactivate(user.id);
			} else {
				await userService.update(user.id, { active: true });
			}
			await load();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'İşlem başarısız.';
		}
	}

	async function createTeam(e: SubmitEvent) {
		e.preventDefault();
		if (teamBusy || !teamName.trim()) return;
		teamBusy = true;
		error = null;
		try {
			await teamAdminService.create({ name: teamName.trim() });
			teamName = '';
			await load();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Takım oluşturulamadı.';
		} finally {
			teamBusy = false;
		}
	}

	async function toggleTeam(teamId: string) {
		if (expandedTeamId === teamId) {
			expandedTeamId = null;
			return;
		}
		expandedTeamId = teamId;
		memberUserId = '';
		try {
			const members = await teamAdminService.listMembers(teamId);
			teamMembers = { ...teamMembers, [teamId]: members };
		} catch {
			teamMembers = { ...teamMembers, [teamId]: [] };
		}
	}

	async function addMember(teamId: string) {
		if (!memberUserId) return;
		error = null;
		try {
			await teamAdminService.addMember(teamId, memberUserId);
			const members = await teamAdminService.listMembers(teamId);
			teamMembers = { ...teamMembers, [teamId]: members };
			memberUserId = '';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Üye eklenemedi.';
		}
	}

	async function removeMember(teamId: string, userId: string) {
		error = null;
		try {
			await teamAdminService.removeMember(teamId, userId);
			const members = await teamAdminService.listMembers(teamId);
			teamMembers = { ...teamMembers, [teamId]: members };
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Üye çıkarılamadı.';
		}
	}

	function memberName(userId: string): string {
		return users.find((u) => u.id === userId)?.full_name ?? userId.slice(0, 8);
	}

	onMount(load);

	async function createTemplate(e: SubmitEvent) {
		e.preventDefault();
		if (busy || !tplName.trim() || !tplCode.trim()) return;
		busy = true;
		error = null;
		try {
			await processTemplateService.create({ name: tplName.trim(), code: tplCode.trim() });
			tplName = '';
			tplCode = '';
			await load();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Şablon oluşturulamadı.';
		} finally {
			busy = false;
		}
	}

	function addStep() {
		if (!grpSelected) return;
		grpSteps = [...grpSteps, { template_id: grpSelected, deps: [] }];
		grpSelected = '';
	}

	function removeStep(i: number) {
		grpSteps = grpSteps.filter((_, idx) => idx !== i);
	}

	/** Her adımı bir öncekine bağımlı yapar (lineer akış). */
	function makeLinear() {
		grpSteps = grpSteps.map((s, i) => ({ ...s, deps: i === 0 ? [] : [i - 1] }));
	}

	async function createGroup(e: SubmitEvent) {
		e.preventDefault();
		if (busy || !grpName.trim() || grpSteps.length === 0) return;
		busy = true;
		error = null;
		try {
			const dependencies: { step: number; depends_on: number }[] = [];
			grpSteps.forEach((s, i) => {
				for (const dep of s.deps) dependencies.push({ step: i, depends_on: dep });
			});
			await processGroupService.create({
				name: grpName.trim(),
				steps: grpSteps.map((s) => ({ template_id: s.template_id })),
				dependencies
			});
			grpName = '';
			grpSteps = [];
			await load();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Grup oluşturulamadı.';
		} finally {
			busy = false;
		}
	}

	function tplName_(id: string): string {
		return templates.find((t) => t.id === id)?.name ?? '?';
	}
</script>

<svelte:head>
	<title>Yönetim — Canlı Atölye</title>
</svelte:head>

<div class="mx-auto max-w-5xl space-y-6">
	<div>
		<h1 class="text-xl font-semibold text-gray-900 dark:text-white">Yönetim</h1>
		<p class="mt-0.5 text-sm text-gray-500 dark:text-gray-400">
			Süreç şablonları ve grupları workspace seviyesinde tanımlanır; kod değişikliği gerekmez.
		</p>
	</div>

	{#if !isAdmin}
		<div class="rounded-lg border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-800 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-300">
			Bu sayfa yalnızca ADMIN rolü içindir.
		</div>
	{:else}
		{#if error}
			<div class="rounded-lg border border-red-300 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300">
				{error}
			</div>
		{/if}

		<!-- Kullanıcılar -->
		<section class="rounded-lg border border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50">
			<header class="border-b border-gray-200 px-4 py-3 dark:border-gray-700">
				<h2 class="text-sm font-semibold text-gray-900 dark:text-white">Kullanıcılar</h2>
				<p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
					Çalışanları ekleyin; rolüne göre yetkileri otomatik belirlenir. Şifre en az 8 karakter.
				</p>
			</header>
			<div class="p-4">
				<form class="mb-4 grid gap-2 sm:grid-cols-5" onsubmit={createUser}>
					<div>
						<label class="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-300" for="u-name">Ad Soyad</label>
						<input id="u-name" class="w-full rounded-lg border border-gray-300 bg-white p-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" placeholder="Ahmet Kesici" bind:value={userName} required />
					</div>
					<div>
						<label class="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-300" for="u-email">E-posta</label>
						<input id="u-email" type="email" class="w-full rounded-lg border border-gray-300 bg-white p-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" placeholder="ahmet@firma.local" bind:value={userEmail} required />
					</div>
					<div>
						<label class="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-300" for="u-pass">Şifre</label>
						<input id="u-pass" type="text" class="w-full rounded-lg border border-gray-300 bg-white p-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" placeholder="min. 8 karakter" bind:value={userPassword} required minlength={8} />
					</div>
					<div>
						<label class="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-300" for="u-role">Rol</label>
						<select id="u-role" class="w-full rounded-lg border border-gray-300 bg-white p-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={userRole}>
							<option value="WORKER">Çalışan</option>
							<option value="TEAM_LEADER">Takım Lideri</option>
							<option value="PROJECT_MANAGER">Proje Yöneticisi</option>
							<option value="VIEWER">İzleyici</option>
							<option value="ADMIN">Yönetici</option>
						</select>
					</div>
					<div class="flex items-end">
						<AppButton type="submit" disabled={userBusy || !userName.trim() || !userEmail.trim() || userPassword.length < 8}>
							{userBusy ? 'Ekleniyor…' : 'Ekle'}
						</AppButton>
					</div>
				</form>

				{#if users.length === 0}
					<p class="text-sm text-gray-400">Henüz kullanıcı yok (sadece siz).</p>
				{:else}
					<div class="overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-gray-200 text-left text-xs uppercase tracking-wide text-gray-400 dark:border-gray-700">
									<th class="py-2 pr-3">Ad</th>
									<th class="px-3 py-2">E-posta</th>
									<th class="px-3 py-2">Rol</th>
									<th class="px-3 py-2">Durum</th>
									<th class="px-3 py-2 text-right">İşlem</th>
								</tr>
							</thead>
							<tbody>
								{#each users as user (user.id)}
									<tr class="border-b border-gray-100 last:border-0 dark:border-gray-700/50 {user.active ? '' : 'opacity-50'}">
										<td class="py-2 pr-3 font-medium text-gray-900 dark:text-white">{user.full_name}</td>
										<td class="px-3 py-2 text-gray-500">{user.email}</td>
										<td class="px-3 py-2">
											{#if user.id === session.me?.user.id}
												<span class="rounded bg-blue-100 px-1.5 py-0.5 text-[10px] font-semibold text-blue-700 dark:bg-blue-900/40 dark:text-blue-300">SİZ</span>
												{ROLE_LABELS[user.role]}
											{:else}
												<select
													class="rounded border border-gray-300 bg-white px-1.5 py-1 text-xs dark:border-gray-600 dark:bg-gray-700 dark:text-white"
													value={user.role}
													onchange={(e) => void changeRole(user, (e.target as HTMLSelectElement).value as UserDto['role'])}
												>
													{#each Object.entries(ROLE_LABELS) as [value, label] (value)}
														<option {value}>{label}</option>
													{/each}
												</select>
											{/if}
										</td>
										<td class="px-3 py-2">
											<span class="{user.active ? 'text-green-600 dark:text-green-400' : 'text-gray-400'}">{user.active ? 'aktif' : 'pasif'}</span>
										</td>
										<td class="px-3 py-2 text-right">
											{#if user.id !== session.me?.user.id}
												<button
													type="button"
													class="rounded px-2 py-1 text-xs font-medium {user.active ? 'text-red-600 hover:bg-red-50 dark:hover:bg-red-950/30' : 'text-green-600 hover:bg-green-50 dark:hover:bg-green-950/30'}"
													onclick={() => void toggleActive(user)}
												>
													{user.active ? 'Pasifleştir' : 'Aktifleştir'}
												</button>
											{/if}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>
		</section>

		<!-- Takımlar -->
		<section class="rounded-lg border border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50">
			<header class="border-b border-gray-200 px-4 py-3 dark:border-gray-700">
				<h2 class="text-sm font-semibold text-gray-900 dark:text-white">Takımlar</h2>
				<p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
					Takıma atanan işleri tüm üyeler görür ve işletebilir.
				</p>
			</header>
			<div class="p-4">
				<form class="mb-4 flex items-end gap-2" onsubmit={createTeam}>
					<div class="flex-1">
						<label class="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-300" for="t-name">Takım adı</label>
						<input id="t-name" class="w-full rounded-lg border border-gray-300 bg-white p-2 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" placeholder="Kesim Ekibi" bind:value={teamName} required />
					</div>
					<AppButton type="submit" disabled={teamBusy || !teamName.trim()}>Oluştur</AppButton>
				</form>

				{#if teams.length === 0}
					<p class="text-sm text-gray-400">Henüz takım yok.</p>
				{:else}
					<ul class="space-y-2">
						{#each teams as team (team.id)}
							<li class="rounded-lg border border-gray-200 dark:border-gray-700">
								<button
									type="button"
									class="flex w-full items-center justify-between px-3 py-2.5 text-left"
									onclick={() => void toggleTeam(team.id)}
								>
									<span class="text-sm font-medium text-gray-900 dark:text-white">{team.name}</span>
									<span class="text-xs text-gray-400">{expandedTeamId === team.id ? '▲' : '▼'} {(teamMembers[team.id] ?? []).length} üye</span>
								</button>
								{#if expandedTeamId === team.id}
									<div class="border-t border-gray-100 px-3 py-2.5 dark:border-gray-700">
										<ul class="mb-2 space-y-1">
											{#each teamMembers[team.id] ?? [] as member (member.user_id)}
												<li class="flex items-center justify-between text-sm">
													<span class="text-gray-800 dark:text-gray-200">
														{memberName(member.user_id)}
														{#if member.role === 'LEADER'}<span class="ml-1 rounded bg-amber-100 px-1 text-[10px] text-amber-700 dark:bg-amber-900/40 dark:text-amber-300">lider</span>{/if}
													</span>
													<button type="button" class="text-xs text-red-500 hover:underline" onclick={() => void removeMember(team.id, member.user_id)}>çıkar</button>
												</li>
											{:else}
												<li class="text-xs text-gray-400">Üye yok.</li>
											{/each}
										</ul>
										<div class="flex gap-1.5">
											<select class="flex-1 rounded-md border border-gray-300 bg-white p-1.5 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white" bind:value={memberUserId}>
												<option value="">Üye seçin…</option>
												{#each users.filter((u) => u.active) as user (user.id)}
													<option value={user.id}>{user.full_name}</option>
												{/each}
											</select>
											<AppButton size="sm" disabled={!memberUserId} onclick={() => void addMember(team.id)}>Ekle</AppButton>
										</div>
									</div>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</section>

		<!-- Denetim kayıtları -->
		{#if audit.length > 0}
		<section class="rounded-lg border border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50">
			<header class="border-b border-gray-200 px-4 py-3 dark:border-gray-700">
				<h2 class="text-sm font-semibold text-gray-900 dark:text-white">Denetim Kayıtları</h2>
				<p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">Son 50 yönetimsel işlem (değiştirilemez).</p>
			</header>
			<ul class="max-h-64 divide-y divide-gray-100 overflow-y-auto px-4 dark:divide-gray-700">
				{#each audit as a (a.id)}
					<li class="flex items-center gap-3 py-2 text-sm">
						<span class="w-14 shrink-0 text-xs text-gray-400 tabular-nums">{new Date(a.timestamp).toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit' })}</span>
						<span class="text-xs text-gray-400">{new Date(a.timestamp).toLocaleDateString('tr-TR', { day: 'numeric', month: 'short' })}</span>
						<span class="font-medium text-gray-800 dark:text-gray-200">{a.full_name}</span>
						<span class="rounded bg-gray-100 px-1.5 py-0.5 font-mono text-[10px] text-gray-600 dark:bg-gray-700 dark:text-gray-300">{a.action}</span>
					</li>
				{/each}
			</ul>
		</section>
		{/if}

		<!-- Şablonlar -->
		<section class="rounded-lg border border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50">
			<header class="border-b border-gray-200 px-4 py-3 dark:border-gray-700">
				<h2 class="text-sm font-semibold text-gray-900 dark:text-white">Süreç Şablonları</h2>
				<p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
					Örn: Kesim, İmalat, Kalite Kontrol, Nakliye, Montaj…
				</p>
			</header>
			<div class="grid gap-4 p-4 md:grid-cols-2">
				<form class="flex items-end gap-2" onsubmit={createTemplate}>
					<div class="flex-1">
						<Label for="tpl-name">Ad</Label>
						<Input id="tpl-name" placeholder="Kesim" bind:value={tplName} required />
					</div>
					<div class="w-24">
						<Label for="tpl-code">Kod</Label>
						<Input id="tpl-code" placeholder="KSM" bind:value={tplCode} required />
					</div>
					<AppButton type="submit" disabled={busy || !tplName.trim() || !tplCode.trim()}>Ekle</AppButton>
				</form>
				{#if templates.length === 0}
					<p class="self-center text-sm text-gray-400 dark:text-gray-500">Henüz şablon yok.</p>
				{:else}
					<ul class="flex flex-wrap gap-1.5 self-center">
						{#each templates as t (t.id)}
							<li class="rounded-md border border-gray-200 bg-gray-50 px-2 py-1 text-xs dark:border-gray-600 dark:bg-gray-700/50">
								{t.name} <span class="text-gray-400">({t.code})</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</section>

		<!-- Gruplar -->
		<section class="rounded-lg border border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-800/50">
			<header class="border-b border-gray-200 px-4 py-3 dark:border-gray-700">
				<h2 class="text-sm font-semibold text-gray-900 dark:text-white">Süreç Grupları</h2>
				<p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
					Bir iş kalemine uygulanacak adım dizisi + bağımlılıklar.
				</p>
			</header>
			<form class="space-y-4 p-4" onsubmit={createGroup}>
				<div class="flex items-end gap-2">
					<div class="flex-1">
						<Label for="grp-name">Grup adı</Label>
						<Input id="grp-name" placeholder="Standart Tezgah Süreci" bind:value={grpName} required />
					</div>
				</div>

				<!-- Adım ekleme -->
				<div class="flex items-end gap-2">
					<div class="flex-1">
						<Label for="grp-tpl">Adım ekle (sıralı)</Label>
						<Select id="grp-tpl" bind:value={grpSelected}>
							<option value="">Şablon seçin…</option>
							{#each templates as t (t.id)}
								<option value={t.id}>{t.name} ({t.code})</option>
							{/each}
						</Select>
					</div>
					<AppButton color="alternative" onclick={addStep} disabled={!grpSelected}>Adım Ekle</AppButton>
					{#if grpSteps.length > 1}
						<AppButton color="alternative" onclick={makeLinear}>Lineer Yap</AppButton>
					{/if}
				</div>

				<!-- Adım listesi + bağımlılıklar -->
				{#if grpSteps.length > 0}
					<ul class="space-y-2">
						{#each grpSteps as step, i (i)}
							<li class="flex flex-wrap items-center gap-2 rounded-lg border border-gray-200 bg-gray-50 px-3 py-2 dark:border-gray-700 dark:bg-gray-800/60">
								<span class="w-6 text-xs font-semibold text-gray-400">{i + 1}</span>
								<span class="text-sm font-medium text-gray-900 dark:text-white">{tplName_(step.template_id)}</span>
								{#if i > 0}
									<div class="ms-auto flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400">
										<span>şunlar tamamlanınca başlar:</span>
										<select
											class="rounded-md border border-gray-300 bg-white p-1 text-xs dark:border-gray-600 dark:bg-gray-700 dark:text-white"
											multiple
											size="1"
											value={step.deps}
											onchange={(e) => {
												const selected = Array.from((e.target as HTMLSelectElement).selectedOptions).map((o) => Number(o.value));
												grpSteps = grpSteps.map((s, idx) => (idx === i ? { ...s, deps: selected } : s));
											}}
										>
											{#each grpSteps.slice(0, i) as _, j (j)}
												<option value={j}>{j + 1}. {tplName_(grpSteps[j].template_id)}</option>
											{/each}
										</select>
									</div>
								{:else}
									<span class="ms-auto text-xs text-gray-400">bağımlılıksız</span>
								{/if}
								<button type="button" class="rounded p-1 text-gray-400 hover:bg-red-50 hover:text-red-600" title="Adımı çıkar" onclick={() => removeStep(i)}>✕</button>
							</li>
						{/each}
					</ul>
				{/if}

				<div class="flex justify-end">
					<AppButton type="submit" disabled={busy || !grpName.trim() || grpSteps.length === 0}>
						{busy ? 'Oluşturuluyor…' : 'Grup Oluştur'}
					</AppButton>
				</div>
			</form>

			{#if groups.length > 0}
				<div class="border-t border-gray-200 px-4 py-3 dark:border-gray-700">
					<div class="text-xs font-medium text-gray-500 dark:text-gray-400">Tanımlı gruplar:</div>
					<ul class="mt-1.5 flex flex-wrap gap-1.5">
						{#each groups as g (g.id)}
							<li class="rounded-md border border-blue-200 bg-blue-50 px-2 py-1 text-xs text-blue-700 dark:border-blue-800 dark:bg-blue-900/30 dark:text-blue-300">
								{g.name}
							</li>
						{/each}
					</ul>
				</div>
			{/if}
		</section>
	{/if}
</div>
