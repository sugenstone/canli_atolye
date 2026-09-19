<script lang="ts">
	/**
	 * Authenticated shell: sidebar + topbar (docs/architecture.md §3.1, §48).
	 * Mobil: sidebar drawer'a dönüşür; gezinme Faz 1'de tek aktif sayfa (Dashboard),
	 * diğerleri "yakında" durumundadır.
	 */
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Drawer, Sidebar, SidebarGroup, SidebarItem, SidebarBrand } from 'flowbite-svelte';
	import { session } from '$lib/stores/session.svelte';
	import {
		notificationService,
		search as doSearch,
		type NotificationDto,
		type SearchHit
	} from '$lib/services/api/insights';

	let { children } = $props();

	let sidebarOpen = $state(false);

	// Auth guard: oturumu backend'den doğrula; geçersizse login'e dön.
	// (hooks.server.ts yalnızca cookie varlığına bakar; gerçek doğrulama /auth/me)
	onMount(() => {
		if (session.isAuthenticated) {
			session.loading = false;
			return;
		}
		void session.load().then((me) => {
			if (!me) goto('/login');
		});
	});

	const nav = [
		{ href: '/dashboard', label: 'Dashboard', icon: 'grid', enabled: true },
		{ href: '/projects', label: 'Projeler', icon: 'folder', enabled: true },
		{ href: '/my-work', label: 'Benim İşlerim', icon: 'user', enabled: true },
		{ href: '/reports', label: 'Raporlar', icon: 'chart', enabled: false },
		{ href: '/admin', label: 'Yönetim', icon: 'cog', enabled: true, adminOnly: true }
	] as const;

	const visibleNav = $derived(nav.filter((item) => !('adminOnly' in item && item.adminOnly) || session.isAdmin));

	const me = $derived(session.me);
	const currentPath = $derived(page.url.pathname);

	function isActive(href: string): boolean {
		return currentPath === href || currentPath.startsWith(href + '/');
	}

	// Bildirimler
	let notifOpen = $state(false);
	let notifications = $state<NotificationDto[]>([]);
	const unreadCount = $derived(notifications.filter((n) => !n.read_at).length);

	async function toggleNotifications() {
		notifOpen = !notifOpen;
		if (notifOpen) {
			notifications = await notificationService.list().catch(() => []);
		}
	}

	async function readNotification(id: string) {
		await notificationService.read(id).catch(() => {});
		notifications = await notificationService.list().catch(() => notifications);
	}

	// Global arama
	let searchQ = $state('');
	let searchHits = $state<SearchHit[] | null>(null);
	let searchBusy = $state(false);
	let searchOpen = $state(false);

	async function runSearch() {
		if (searchQ.trim().length < 2) {
			searchHits = null;
			return;
		}
		searchBusy = true;
		searchHits = await doSearch(searchQ.trim()).catch(() => []);
		searchBusy = false;
	}

	function goHit(hit: SearchHit) {
		searchOpen = false;
		searchHits = null;
		searchQ = '';
		void goto(`/projects/${hit.project_id}/tree?sectionId=${hit.section_id}`);
	}

	// Bildirimleri periyodik çek
	$effect(() => {
		const load = () => void notificationService.list().then((n) => (notifications = n)).catch(() => {});
		load();
		const t = setInterval(load, 30_000);
		return () => clearInterval(t);
	});

	async function logout() {
		await session.logout();
		await goto('/login');
	}
</script>

<div class="flex min-h-screen bg-gray-50 dark:bg-gray-900">
	<!-- Desktop sidebar -->
	<aside class="hidden w-64 shrink-0 border-r border-gray-200 bg-white md:block dark:border-gray-800 dark:bg-gray-800/50">
		<Sidebar>
			<SidebarBrand href="/dashboard" class="mb-2">
				<span class="flex items-center gap-2.5 px-2 py-1">
					<span
						class="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-600 text-sm font-bold text-white"
					>
						CA
					</span>
					<span class="text-base font-semibold text-gray-900 dark:text-white">Canlı Atölye</span>
				</span>
			</SidebarBrand>
			<SidebarGroup>
				{#each visibleNav as item (item.href)}
					<SidebarItem
						href={item.enabled ? item.href : undefined}
						label={item.label}
						active={item.enabled && isActive(item.href)}
						class={item.enabled ? '' : 'cursor-not-allowed opacity-50'}
						onclick={item.enabled ? undefined : ((e: Event) => e.preventDefault())}
					>
						{#if !item.enabled}
							<span class="ms-1 rounded bg-gray-100 px-1.5 py-0.5 text-[10px] text-gray-500 dark:bg-gray-700 dark:text-gray-400">
								yakında
							</span>
						{/if}
					</SidebarItem>
				{/each}
			</SidebarGroup>
		</Sidebar>
	</aside>

	<!-- Mobile drawer -->
	<Drawer bind:open={sidebarOpen} placement="left" class="w-64 p-0">
		<div class="flex h-full flex-col">
			<div class="flex items-center gap-2.5 border-b border-gray-200 px-4 py-4 dark:border-gray-700">
				<span
					class="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-600 text-sm font-bold text-white"
				>
					CA
				</span>
				<span class="text-base font-semibold text-gray-900 dark:text-white">Canlı Atölye</span>
			</div>
			<nav class="flex flex-col gap-1 p-3">
				{#each visibleNav as item (item.href)}
					<a
						href={item.enabled ? item.href : '#'}
						class="flex items-center justify-between rounded-lg px-3 py-2 text-sm font-medium
							{item.enabled && isActive(item.href)
								? 'bg-blue-50 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
								: 'text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-800'}"
						onclick={() => item.enabled && (sidebarOpen = false)}
					>
						{item.label}
						{#if !item.enabled}
							<span class="rounded bg-gray-100 px-1.5 py-0.5 text-[10px] text-gray-500 dark:bg-gray-700 dark:text-gray-400">
								yakında
							</span>
						{/if}
					</a>
				{/each}
			</nav>
		</div>
	</Drawer>

	<!-- Main -->
	<div class="flex min-w-0 flex-1 flex-col">
		<header
			class="sticky top-0 z-10 flex h-14 items-center justify-between border-b border-gray-200 bg-white px-4 dark:border-gray-800 dark:bg-gray-900"
		>
			<div class="flex items-center gap-3">
				<!-- Mobil hamburger -->
				<button
					type="button"
					class="rounded-lg p-2 text-gray-500 hover:bg-gray-100 md:hidden dark:text-gray-400 dark:hover:bg-gray-800"
					aria-label="Menüyü aç"
					onclick={() => (sidebarOpen = true)}
				>
					<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16M4 18h16" />
					</svg>
				</button>
				<span class="text-sm font-medium text-gray-600 dark:text-gray-400">
					{me?.workspace.name ?? ''}
				</span>
			</div>

			<div class="flex items-center gap-2">
				<!-- Global arama -->
				<div class="relative hidden md:block">
					<input
						type="search"
						class="w-52 rounded-lg border border-gray-300 bg-gray-50 p-1.5 pl-8 text-sm dark:border-gray-600 dark:bg-gray-800 dark:text-white"
						placeholder="Ara: daire, tezgah…"
						bind:value={searchQ}
						oninput={() => { searchOpen = true; void runSearch(); }}
						onfocus={() => (searchOpen = true)}
					/>
					<svg class="pointer-events-none absolute left-2.5 top-2.5 h-4 w-4 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-4.35-4.35M17 10a7 7 0 11-14 0 7 7 0 0114 0z" />
					</svg>
					{#if searchOpen && searchHits !== null}
						<div class="absolute right-0 top-11 z-30 w-80 max-h-80 overflow-y-auto rounded-lg border border-gray-200 bg-white shadow-lg dark:border-gray-700 dark:bg-gray-800">
							{#each searchHits as hit (hit.kind + hit.section_id + (hit.work_item_id ?? ''))}
								<button type="button" class="block w-full px-3 py-2 text-left hover:bg-gray-50 dark:hover:bg-gray-700/50" onclick={() => goHit(hit)}>
									<div class="text-sm font-medium text-gray-900 dark:text-white">{hit.title}</div>
									<div class="text-xs text-gray-400">{hit.subtitle}</div>
								</button>
							{:else}
								<div class="px-3 py-4 text-sm text-gray-400">Sonuç yok.</div>
							{/each}
						</div>
					{/if}
				</div>

				<!-- Bildirim zili -->
				<div class="relative">
					<button
						type="button"
						class="relative rounded-lg p-2 text-gray-500 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800"
						aria-label="Bildirimler"
						onclick={toggleNotifications}
					>
						<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M15 17h5l-1.4-1.4A2 2 0 0118 14.2V11a6 6 0 00-4-5.7V5a2 2 0 10-4 0v.3A6 6 0 006 11v3.2a2 2 0 01-.6 1.4L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
						</svg>
						{#if unreadCount > 0}
							<span class="absolute -right-0.5 -top-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-500 px-1 text-[10px] font-bold text-white">
								{unreadCount}
							</span>
						{/if}
					</button>
					{#if notifOpen}
						<div class="absolute right-0 top-11 z-30 w-80 max-h-96 overflow-y-auto rounded-lg border border-gray-200 bg-white shadow-lg dark:border-gray-700 dark:bg-gray-800">
							<div class="flex items-center justify-between border-b border-gray-100 px-3 py-2 dark:border-gray-700">
								<span class="text-xs font-semibold uppercase text-gray-500">Bildirimler</span>
								<button type="button" class="text-[11px] font-medium text-blue-600 hover:underline" onclick={async () => { await notificationService.readAll().catch(() => {}); notifications = await notificationService.list().catch(() => notifications); }}>Tümünü okundu işaretle</button>
							</div>
							{#each notifications as n (n.id)}
								<button type="button" class="block w-full px-3 py-2.5 text-left hover:bg-gray-50 dark:hover:bg-gray-700/50 {n.read_at ? 'opacity-60' : ''}" onclick={() => void readNotification(n.id)}>
									<div class="flex items-center gap-2">
										{#if !n.read_at}<span class="h-2 w-2 shrink-0 rounded-full bg-blue-500"></span>{/if}
										<span class="text-sm font-medium text-gray-900 dark:text-white">{n.title}</span>
									</div>
									<div class="text-xs text-gray-500">{n.message}</div>
									<div class="text-[10px] text-gray-400">{new Date(n.created_at).toLocaleString('tr-TR')}</div>
								</button>
							{:else}
								<div class="px-3 py-6 text-center text-sm text-gray-400">Bildirim yok.</div>
							{/each}
						</div>
					{/if}
				</div>

				<span class="hidden text-sm text-gray-600 lg:block dark:text-gray-400">
					{me?.user.full_name}
					<span class="ms-1.5 rounded bg-gray-100 px-1.5 py-0.5 text-[10px] font-medium text-gray-500 dark:bg-gray-800 dark:text-gray-400">
						{me?.user.role}
					</span>
				</span>
				<button
					type="button"
					class="rounded-lg p-2 text-gray-500 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800"
					aria-label="Çıkış yap"
					title="Çıkış yap"
					onclick={logout}
				>
					<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15m3 0l3-3m0 0l-3-3m3 3H9"
						/>
					</svg>
				</button>
			</div>
		</header>

		<main class="flex-1 p-4 md:p-6">
			{#if session.loading}
				<!-- Loading skeleton (docs/architecture.md §53) -->
				<div class="animate-pulse space-y-4">
					<div class="h-8 w-64 rounded bg-gray-200 dark:bg-gray-700"></div>
					<div class="h-32 rounded-lg bg-gray-200 dark:bg-gray-700"></div>
					<div class="h-32 rounded-lg bg-gray-200 dark:bg-gray-700"></div>
				</div>
			{:else}
				{@render children?.()}
			{/if}
		</main>
	</div>
</div>
