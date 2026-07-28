<!-- Canonical source; copied into consuming repositories by scripts/sync.mjs. -->
<script lang="ts">
	let {
		sourceHref = '',
		projectName = '',
		localNavigation = false,
		notice = '',
		additionalLinks = []
	}: {
		sourceHref?: string;
		projectName?: string;
		localNavigation?: boolean;
		notice?: string;
		additionalLinks?: Array<{ href: string; label: string }>;
	} = $props();
	const year = new Date().getFullYear();
</script>

<footer class="border-t border-gray-200 bg-[#f9fbff] px-4 py-3 text-center text-sm text-gray-500 dark:border-gray-800 dark:bg-[#111827] dark:text-gray-400">
	{#if notice}
		<p class="mx-auto mb-2 mt-0 max-w-3xl text-xs leading-relaxed">{notice}</p>
	{/if}
	<p class="m-0 flex flex-wrap items-center justify-center gap-x-2 gap-y-1">
		<span>© {year} Joris Perrenet</span>
		<span aria-hidden="true">·</span>
		<a rel={localNavigation ? undefined : 'external'} class="hover:text-blue-600 hover:underline dark:hover:text-blue-400" href={localNavigation ? '/about/' : 'https://jorisperrenet.com/about/'}>About</a>
		{#if sourceHref}
			<span aria-hidden="true">·</span>
			<a class="hover:text-blue-600 hover:underline dark:hover:text-blue-400" href={sourceHref}>Source{projectName ? ` for ${projectName}` : ''}</a>
		{/if}
		{#each additionalLinks as link}
			<span aria-hidden="true">·</span>
			<a class="hover:text-blue-600 hover:underline dark:hover:text-blue-400" href={link.href}>{link.label}</a>
		{/each}
	</p>
</footer>
