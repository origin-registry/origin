import { redirect } from '@sveltejs/kit';
import { base } from '$app/paths';
import type { PageLoad } from './$types';

export interface ScanParams {
	/** Namespace holding the report manifest. */
	path: string;
	/** Digest of the report manifest. */
	digest: string;
}

export const load: PageLoad = ({ params }): ScanParams => {
	const at = params.path.lastIndexOf('@');
	if (at === -1) {
		redirect(307, `${base}/`);
	}
	return { path: params.path.slice(0, at), digest: params.path.slice(at + 1) };
};
