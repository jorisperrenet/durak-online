<!-- Canonical source; copied into consuming repositories by scripts/sync.mjs. -->
<script lang="ts">
	let {
		title,
		description,
		canonical,
		image = 'https://jorisperrenet.com/profile.jpg',
		imageWidth = 1200,
		imageHeight = 630,
		type = 'website'
	}: {
		title: string;
		description: string;
		canonical: string;
		image?: string;
		imageWidth?: number;
		imageHeight?: number;
		type?: 'website' | 'article';
	} = $props();
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={canonical} />
	<meta property="og:type" content={type} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={canonical} />
	<meta property="og:image" content={image} />
	<meta property="og:image:width" content={String(imageWidth)} />
	<meta property="og:image:height" content={String(imageHeight)} />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<meta name="twitter:image" content={image} />
</svelte:head>
