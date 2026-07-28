// Shared navigation state.
export type SiteSection = 'home' | 'projects' | 'about' | 'blog' | 'project' | '';

export type SiteMenuItem = {
	href: string;
	label: string;
	current?: boolean;
};

export type SiteProject = {
	id: string;
	label: string;
	href: string;
};
