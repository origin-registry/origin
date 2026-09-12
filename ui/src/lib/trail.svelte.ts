/** The breadcrumb trail the top bar shows; each page sets it through `Breadcrumb`. */
export interface TrailItem {
	label: string;
	href?: string;
}

export const trail: { items: TrailItem[] } = $state({ items: [] });
