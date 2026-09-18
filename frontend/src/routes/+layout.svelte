<script lang="ts">
	// Kök layout: design system kökünü yükle (docs/architecture.md §3, §4).
	// KRİTİK: app.css import'u buradan çıkarılırsa Tailwind hiç yüklenmez.
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
