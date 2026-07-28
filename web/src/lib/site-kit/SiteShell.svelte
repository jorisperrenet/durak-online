<!-- Canonical source; copied into consuming repositories by scripts/sync.mjs. -->
<script lang="ts">
	import SiteFooter from './SiteFooter.svelte';
	import SiteHeader from './SiteHeader.svelte';
	import type { SiteMenuItem, SiteSection } from './types';
	import type { Snippet } from 'svelte';

	let {
		children,
		active = 'project',
		projectName = '',
		currentProjectId = '',
		projectHref = '',
		sourceHref = '',
		workspaceClass = '',
		localNavigation = false,
		brandLogo = false,
		logoSrc = '/personal-logo.svg',
		menuLabel = '',
		menuItems = [],
		notice = '',
		additionalLinks = []
	}: {
		children: Snippet;
		active?: SiteSection;
		projectName?: string;
		currentProjectId?: string;
		projectHref?: string;
		sourceHref?: string;
		workspaceClass?: string;
		localNavigation?: boolean;
		brandLogo?: boolean;
		logoSrc?: string;
		menuLabel?: string;
		menuItems?: SiteMenuItem[];
		notice?: string;
		additionalLinks?: Array<{ href: string; label: string }>;
	} = $props();
</script>

<div class="flex min-h-screen flex-col bg-[#f9fbff] text-gray-900 dark:bg-[#111827] dark:text-gray-100">
	<SiteHeader {active} {projectName} {currentProjectId} {projectHref} {localNavigation} {brandLogo} {logoSrc} {menuLabel} {menuItems} />
	<main class={`flex-1 ${workspaceClass}`}>{@render children()}</main>
	<SiteFooter {sourceHref} {projectName} {localNavigation} {notice} {additionalLinks} />
</div>
