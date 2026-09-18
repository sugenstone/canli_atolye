<script lang="ts">
	/**
	 * StatusBadge — tüm durum görselleştirmeleri bu bileşenden geçer.
	 * Renk/etiket config/status.ts'ten gelir; burada renk yazılmaz.
	 */
	import { statusToken, PROJECT_STATUS, PROCESS_STATUS } from '$lib/config/status';

	let {
		status,
		kind = 'project',
		size = 'sm'
	}: {
		status: string;
		kind?: 'project' | 'process';
		size?: 'xs' | 'sm';
	} = $props();

	const token = $derived(
		statusToken(kind === 'process' ? PROCESS_STATUS : PROJECT_STATUS, status)
	);
	const sizeClass = $derived(
		size === 'xs' ? 'text-[10px] px-1.5 py-0.5' : 'text-xs px-2.5 py-0.5'
	);
</script>

<span
	class="inline-flex items-center gap-1.5 rounded-md font-medium whitespace-nowrap {token.badgeClass} {sizeClass}"
	title="{token.label}"
>
	<span class="h-1.5 w-1.5 rounded-full {token.dotClass}"></span>
	{token.label}
</span>
