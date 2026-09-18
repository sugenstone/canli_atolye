<script lang="ts">
	/** UI wrapper: Flowbite Modal sarar (docs/architecture.md §4.2). */
	import { Modal } from 'flowbite-svelte';

	let {
		open = $bindable(false),
		title,
		children,
		footer
	}: {
		open?: boolean;
		title: string;
		children?: import('svelte').Snippet;
		footer?: import('svelte').Snippet;
	} = $props();
</script>

<Modal bind:open {title} size="md" autoclose={false}>
	{@render children?.()}
	{#if footer}
		<div class="mt-5 flex justify-end gap-2">
			{@render footer()}
		</div>
	{/if}
</Modal>
