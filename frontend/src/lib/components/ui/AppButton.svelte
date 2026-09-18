<script lang="ts">
	/**
	 * UI wrapper: Flowbite Button'ı tek yerde sarar (docs/architecture.md §4.2).
	 * Domain component'ler Flowbite API'sini doğrudan bilmez; Flowbite değişirse
	 * yalnızca bu dosya güncellenir.
	 */
	import { Button } from 'flowbite-svelte';

	let {
		children,
		color = 'primary',
		size = 'md',
		type = 'button',
		disabled = false,
		class: className = '',
		onclick
	}: {
		children?: import('svelte').Snippet;
		color?: 'primary' | 'alternative' | 'light' | 'red' | 'dark';
		size?: 'xs' | 'sm' | 'md' | 'lg';
		type?: 'button' | 'submit';
		disabled?: boolean;
		class?: string;
		onclick?: (e: MouseEvent) => void;
	} = $props();
</script>

<Button {color} {size} {type} {disabled} class={className} {onclick}>
	{@render children?.()}
</Button>
