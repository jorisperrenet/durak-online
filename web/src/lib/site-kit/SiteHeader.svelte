<!-- Canonical source; copied into consuming repositories by scripts/sync.mjs. -->
<script lang="ts">
	import ThemeToggle from './ThemeToggle.svelte';
	import { selectedProjects } from './projects';
	import type { SiteMenuItem, SiteSection } from './types';

	let {
		active = 'project',
		projectName = 'Personal website',
		currentProjectId = 'personal-website',
		projectHref = '',
		localNavigation = false,
		brandLogo = false,
		logoSrc = '/personal-logo.svg',
		menuLabel = '',
		menuItems = []
	}: {
		active?: SiteSection;
		projectName?: string;
		currentProjectId?: string;
		projectHref?: string;
		localNavigation?: boolean;
		brandLogo?: boolean;
		logoSrc?: string;
		menuLabel?: string;
		menuItems?: SiteMenuItem[];
	} = $props();

	const links = $derived<Array<{ href: string; label: string; section: SiteSection }>>([
		{ href: localNavigation ? '/projects/' : 'https://jorisperrenet.com/projects/', label: 'Projects', section: 'projects' },
		{ href: localNavigation ? '/blog/' : 'https://jorisperrenet.com/blog/', label: 'Blog', section: 'blog' },
		{ href: localNavigation ? '/about/' : 'https://jorisperrenet.com/about/', label: 'About', section: 'about' }
	]);

	const personalHref = $derived(localNavigation ? '/' : 'https://jorisperrenet.com/');

</script>

<header data-brand-logo={brandLogo || undefined} data-project-href={projectHref || undefined} class="sticky top-0 z-30 w-full border-b border-gray-200 bg-[#fcfeff]/90 text-gray-900 shadow-sm shadow-gray-950/5 backdrop-blur-md dark:border-gray-800 dark:bg-gray-800/90 dark:text-gray-100">
	<div class="grid min-h-10 w-full grid-cols-[minmax(0,1fr)_auto] items-center gap-2 px-3 py-1 sm:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] sm:px-4">
		<div class="flex min-w-0 items-center">
			<a href={personalHref} aria-label="Personal website home" class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-lg transition-colors hover:bg-gray-100 dark:hover:bg-gray-700">
				<img src={logoSrc} alt="" width="690" height="750" class="h-7 w-7 object-contain dark:invert" />
			</a>
		</div>

		<nav aria-label="Main navigation" class="hidden sm:block">
			<ul class="m-0 flex items-center gap-0.5 p-0">
				{#each links as link}
					<li>
						<a
							href={link.href}
							rel={localNavigation ? undefined : 'external'}
							aria-current={active === link.section ? 'page' : undefined}
							class="block border-b-2 px-3 py-1.5 text-sm font-bold uppercase tracking-wider transition-colors {active === link.section
								? 'border-blue-500 text-black dark:border-blue-400 dark:text-white'
								: 'border-transparent text-gray-600 hover:text-black dark:text-gray-300 dark:hover:text-white'}"
						>{link.label}</a>
					</li>
				{/each}
				{#if menuLabel && menuItems.length}
					<li class="relative">
						<details class="group">
							<summary class="flex cursor-pointer list-none items-center gap-1 border-b-2 border-transparent px-3 py-1.5 text-sm font-bold uppercase tracking-wider text-gray-600 transition-colors hover:text-black dark:text-gray-300 dark:hover:text-white [&::-webkit-details-marker]:hidden">
								<span>{menuLabel}</span>
								<svg viewBox="0 0 20 20" class="h-3.5 w-3.5 transition-transform group-open:rotate-180" aria-hidden="true">
									<path d="m5 7.5 5 5 5-5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
								</svg>
							</summary>
							<div class="absolute right-0 top-full z-50 mt-2 max-h-[70vh] w-64 overflow-y-auto rounded-xl border border-gray-200 bg-white p-1.5 text-left shadow-xl dark:border-gray-700 dark:bg-gray-800">
								{#each menuItems as item}
									<a href={item.href} aria-current={item.current ? 'page' : undefined} class="block rounded-lg px-3 py-2 text-sm font-medium normal-case tracking-normal no-underline transition-colors {item.current ? 'bg-blue-50 text-blue-700 dark:bg-blue-950/50 dark:text-blue-300' : 'text-gray-700 hover:bg-gray-100 hover:text-black dark:text-gray-200 dark:hover:bg-gray-700 dark:hover:text-white'}">{item.label}</a>
								{/each}
							</div>
						</details>
					</li>
				{/if}
			</ul>
		</nav>

		<div class="flex items-center justify-end gap-1">
			<ThemeToggle />
			<button
				type="button"
				class="inline-flex h-9 w-9 items-center justify-center rounded-lg text-gray-600 transition-colors hover:bg-gray-100 hover:text-black dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:text-white"
				aria-label="Open site navigation"
				popovertarget="site-navigation-drawer"
			>
				<svg viewBox="0 0 24 24" class="h-5 w-5" aria-hidden="true">
					<path d="M4 7h16M4 12h16M4 17h16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
				</svg>
			</button>
		</div>
	</div>
</header>

	<aside id="site-navigation-drawer" popover="auto" aria-label="Site navigation" class="fixed inset-y-0 left-auto right-0 z-50 m-0 hidden h-dvh w-[min(24rem,92vw)] flex-col border-0 border-l border-gray-200 bg-[#fcfeff] p-0 text-gray-900 shadow-2xl [&:popover-open]:flex [&::backdrop]:bg-gray-950/45 [&::backdrop]:backdrop-blur-[1px] dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100">
		<div class="flex items-center justify-between border-b border-gray-200 px-5 py-4 dark:border-gray-700">
			<div class="min-w-0">
				<p class="m-0 text-xs font-bold uppercase tracking-[0.16em] text-gray-500 dark:text-gray-400">Currently viewing</p>
				<p class="mt-1 truncate text-lg font-bold">{projectName}</p>
			</div>
			<button type="button" aria-label="Close site navigation" popovertarget="site-navigation-drawer" popovertargetaction="hide" class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-gray-600 hover:bg-gray-100 hover:text-black dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white">
				<svg viewBox="0 0 24 24" class="h-5 w-5" aria-hidden="true">
					<path d="m6 6 12 12M18 6 6 18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
				</svg>
			</button>
		</div>

		<div class="overflow-y-auto p-4">
			<nav aria-label="General pages">
				<p class="mb-2 mt-0 px-2 text-xs font-bold uppercase tracking-[0.16em] text-gray-500 dark:text-gray-400">Navigate</p>
				<div class="grid grid-cols-2 gap-1">
					<a href={personalHref} aria-current={currentProjectId === 'personal-website' ? 'page' : undefined} class="rounded-lg px-3 py-2 text-sm font-semibold no-underline {currentProjectId === 'personal-website' ? 'bg-blue-50 text-blue-700 dark:bg-blue-950/50 dark:text-blue-300' : 'hover:bg-gray-100 dark:hover:bg-gray-800'}">Personal website</a>
					{#each links as link}
						<a href={link.href} aria-current={active === link.section ? 'page' : undefined} class="rounded-lg px-3 py-2 text-sm font-semibold no-underline {active === link.section ? 'bg-blue-50 text-blue-700 dark:bg-blue-950/50 dark:text-blue-300' : 'hover:bg-gray-100 dark:hover:bg-gray-800'}">{link.label}</a>
					{/each}
				</div>
			</nav>

			<nav aria-label="Selected projects" class="mt-6">
				<div class="mb-2 flex items-center justify-between px-2">
					<p class="m-0 text-xs font-bold uppercase tracking-[0.16em] text-gray-500 dark:text-gray-400">Selected work</p>
					<a href={localNavigation ? '/projects/' : 'https://jorisperrenet.com/projects/'} class="text-xs font-semibold text-blue-700 hover:underline dark:text-blue-300">View all</a>
				</div>
				<ul class="m-0 grid list-none gap-1 p-0">
					{#each selectedProjects as project}
						<li>
							<a
								href={project.href}
								aria-current={currentProjectId === project.id ? 'page' : undefined}
								class="flex items-center justify-between gap-3 rounded-lg px-3 py-2.5 text-sm font-semibold no-underline transition-colors {currentProjectId === project.id
									? 'bg-blue-50 text-blue-700 dark:bg-blue-950/50 dark:text-blue-300'
									: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
							>
								<span>{project.label}</span>
								{#if currentProjectId === project.id}
									<svg viewBox="0 0 20 20" class="h-4 w-4 shrink-0" aria-hidden="true"><path d="m4 10 4 4 8-8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg>
								{/if}
							</a>
						</li>
					{/each}
				</ul>
			</nav>
		</div>
	</aside>
