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
	const projectsHref = $derived(localNavigation ? '/projects/' : 'https://jorisperrenet.com/projects/');

</script>

<header data-brand-logo={brandLogo || undefined} data-project-href={projectHref || undefined} class="sticky top-0 z-30 w-full border-b border-gray-200 bg-[#fcfeff]/90 text-gray-900 shadow-sm shadow-gray-950/5 backdrop-blur-md dark:border-gray-800 dark:bg-gray-800/90 dark:text-gray-100">
	<div class="grid min-h-10 w-full grid-cols-[minmax(0,1fr)_auto] items-center gap-2 px-3 py-1 sm:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] sm:px-4">
		<div class="flex min-w-0 items-center">
			<a href={personalHref} aria-label="Joris Perrenet — home" class="group inline-flex min-w-0 items-center gap-2 rounded-lg pr-2 transition-colors hover:bg-gray-100 dark:hover:bg-gray-700">
				<span class="inline-flex h-9 w-9 shrink-0 items-center justify-center">
				<img src={logoSrc} alt="" width="690" height="750" class="h-7 w-7 object-contain dark:invert" />
				</span>
				<span class="hidden truncate text-sm font-bold tracking-tight lg:block">Joris Perrenet</span>
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
							class="flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm font-bold transition-colors {active === link.section
								? 'bg-blue-100 text-blue-800 dark:bg-blue-950/70 dark:text-blue-200'
								: 'text-gray-600 hover:bg-gray-100 hover:text-black dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:text-white'}"
						>
							{#if link.section === 'projects'}
								<svg viewBox="0 0 20 20" class="h-4 w-4" aria-hidden="true"><rect x="2.5" y="2.5" width="6" height="6" rx="1.2" fill="currentColor" /><rect x="11.5" y="2.5" width="6" height="6" rx="1.2" fill="currentColor" opacity=".55" /><rect x="2.5" y="11.5" width="6" height="6" rx="1.2" fill="currentColor" opacity=".55" /><rect x="11.5" y="11.5" width="6" height="6" rx="1.2" fill="currentColor" opacity=".3" /></svg>
							{:else if link.section === 'blog'}
								<svg viewBox="0 0 20 20" class="h-4 w-4" aria-hidden="true"><path d="M5 3.5h7l3 3v10H5zM12 3.5v3h3M7.5 10h5M7.5 13h5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
							{:else}
								<svg viewBox="0 0 20 20" class="h-4 w-4" aria-hidden="true"><circle cx="10" cy="6.5" r="3" fill="none" stroke="currentColor" stroke-width="1.5" /><path d="M4.5 16c.5-3 2.4-4.5 5.5-4.5s5 1.5 5.5 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg>
							{/if}
							{link.label}
						</a>
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
					class="inline-flex h-9 items-center justify-center gap-2 rounded-lg border border-gray-200 bg-white px-2.5 text-sm font-bold text-gray-700 shadow-sm transition-colors hover:border-blue-300 hover:bg-blue-50 hover:text-blue-800 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100 dark:hover:border-blue-600 dark:hover:bg-blue-950/50"
					aria-label="Explore all pages and projects"
					popovertarget="site-navigation-drawer"
				>
					<svg viewBox="0 0 24 24" class="h-5 w-5" aria-hidden="true">
						<path d="M4 7h16M4 12h16M4 17h16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
					</svg>
					<span class="hidden lg:inline">Menu</span>
				</button>
		</div>
	</div>
</header>

		<aside id="site-navigation-drawer" popover="auto" aria-label="Explore pages and projects" class="fixed inset-y-0 left-auto right-0 z-50 m-0 hidden h-dvh w-[min(32rem,94vw)] flex-col border-0 border-l border-gray-200 bg-[#fcfeff] p-0 text-gray-900 shadow-2xl [&:popover-open]:flex [&::backdrop]:bg-gray-950/45 [&::backdrop]:backdrop-blur-[1px] dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100">
			<div class="flex items-center justify-between border-b border-gray-200 px-5 py-4 dark:border-gray-700">
				<div class="min-w-0">
					<p class="m-0 text-lg font-bold">Explore</p>
					<p class="m-0 mt-0.5 text-sm text-gray-500 dark:text-gray-400">Pages and projects by Joris Perrenet</p>
			</div>
			<button type="button" aria-label="Close site navigation" popovertarget="site-navigation-drawer" popovertargetaction="hide" class="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-gray-600 hover:bg-gray-100 hover:text-black dark:text-gray-300 dark:hover:bg-gray-800 dark:hover:text-white">
				<svg viewBox="0 0 24 24" class="h-5 w-5" aria-hidden="true">
					<path d="m6 6 12 12M18 6 6 18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
				</svg>
			</button>
		</div>

		<div class="overflow-y-auto p-4">
				<nav aria-label="General pages">
					<p class="mb-2 mt-0 text-xs font-bold uppercase tracking-[0.16em] text-gray-500 dark:text-gray-400">Pages</p>
					<div class="grid grid-cols-2 gap-2">
						<a href={personalHref} aria-current={active === 'home' ? 'page' : undefined} class="group/page flex items-center gap-3 rounded-xl border bg-white p-2.5 text-sm font-bold no-underline shadow-sm transition-all hover:-translate-y-0.5 hover:border-indigo-300 hover:shadow-md dark:bg-gray-800 {active === 'home' ? 'border-indigo-400 ring-2 ring-indigo-400/30 dark:border-indigo-500' : 'border-gray-200 dark:border-gray-700'}">
							<span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-indigo-100 text-indigo-700 transition-colors group-hover/page:bg-indigo-200 dark:bg-indigo-950 dark:text-indigo-300">
								<svg viewBox="0 0 20 20" class="h-5 w-5" aria-hidden="true"><path d="m3 9 7-6 7 6v8H6v-6h8v6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
							</span>
							<span>Home</span>
						</a>
						{#each links as link}
							<a href={link.href} aria-current={active === link.section ? 'page' : undefined} class="group/page flex items-center gap-3 rounded-xl border bg-white p-2.5 text-sm font-bold no-underline shadow-sm transition-all hover:-translate-y-0.5 hover:shadow-md dark:bg-gray-800 {active === link.section ? 'border-blue-400 ring-2 ring-blue-400/30 dark:border-blue-500' : 'border-gray-200 hover:border-blue-300 dark:border-gray-700'}">
								<span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg {link.section === 'projects' ? 'bg-cyan-100 text-cyan-700 dark:bg-cyan-950 dark:text-cyan-300' : link.section === 'blog' ? 'bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-300' : 'bg-rose-100 text-rose-700 dark:bg-rose-950 dark:text-rose-300'}">
								{#if link.section === 'projects'}
									<svg viewBox="0 0 20 20" class="h-5 w-5" aria-hidden="true"><rect x="2.5" y="2.5" width="6" height="6" rx="1.2" fill="currentColor" /><rect x="11.5" y="2.5" width="6" height="6" rx="1.2" fill="currentColor" opacity=".55" /><rect x="2.5" y="11.5" width="6" height="6" rx="1.2" fill="currentColor" opacity=".55" /><rect x="11.5" y="11.5" width="6" height="6" rx="1.2" fill="currentColor" opacity=".3" /></svg>
								{:else if link.section === 'blog'}
									<svg viewBox="0 0 20 20" class="h-5 w-5" aria-hidden="true"><path d="M5 3.5h7l3 3v10H5zM12 3.5v3h3M7.5 10h5M7.5 13h5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
								{:else}
									<svg viewBox="0 0 20 20" class="h-5 w-5" aria-hidden="true"><circle cx="10" cy="6.5" r="3" fill="none" stroke="currentColor" stroke-width="1.5" /><path d="M4.5 16c.5-3 2.4-4.5 5.5-4.5s5 1.5 5.5 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg>
								{/if}
								</span>
								{link.label}
							</a>
						{/each}
				</div>
			</nav>

			<nav aria-label="Selected projects" class="mt-6">
				<div class="mb-2 flex items-center justify-between px-2">
					<p class="m-0 text-xs font-bold uppercase tracking-[0.16em] text-gray-500 dark:text-gray-400">Selected work</p>
					<a href={projectsHref} class="text-xs font-semibold text-blue-700 hover:underline dark:text-blue-300">View all</a>
				</div>
				<ul class="m-0 grid list-none grid-cols-2 gap-2 p-0">
					{#each selectedProjects as project}
						<li>
							<a
								href={project.href}
								aria-current={currentProjectId === project.id ? 'page' : undefined}
								class="group/project relative flex h-24 overflow-hidden rounded-xl border bg-gradient-to-br {project.accent} no-underline shadow-sm transition-all {currentProjectId === project.id ? 'border-blue-500 ring-2 ring-blue-500' : 'border-gray-200 dark:border-gray-700'}"
							>
								{#if project.image}
									<img src={project.image} alt="" class="h-full w-full object-cover transition-transform duration-300 group-hover/project:scale-105" loading="lazy" />
								{:else}
									<span class="m-auto text-3xl font-black text-white/80">{project.label.slice(0, 2)}</span>
								{/if}
								<span class="absolute inset-x-0 bottom-0 bg-gradient-to-t from-gray-950 via-gray-950/80 to-transparent px-2.5 pb-2 pt-6 text-xs font-bold leading-tight text-white">{project.label}</span>
								{#if currentProjectId === project.id}
									<span class="absolute right-1.5 top-1.5 rounded-full bg-blue-600 px-1.5 py-0.5 text-[0.55rem] font-black uppercase tracking-wider text-white shadow">Here</span>
								{/if}
							</a>
						</li>
					{/each}
				</ul>
			</nav>
		</div>
	</aside>
