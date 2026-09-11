<script lang="ts">
	import { base } from '$app/paths';
	import { fetchManifest, fetchRevisions, type Manifest } from '$lib/api';
	import { formatTimeAgo, manifestUrl, pathUrl } from '$lib/utils';
	import Breadcrumb from '$lib/components/Breadcrumb.svelte';
	import Card from '$lib/components/Card.svelte';
	import DigestLink from '$lib/components/DigestLink.svelte';
	import TagList from '$lib/components/TagList.svelte';
	import LoadingState from '$lib/components/LoadingState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import ScanReport from '$lib/components/ScanReport.svelte';
	import type { ScanParams } from './+page';

	let { data }: { data: ScanParams } = $props();
	let manifest = $state<Manifest | null>(null);
	let error = $state<string | null>(null);
	let subjectTags = $state<string[]>([]);

	const subject = $derived(manifest?.subject?.digest ?? null);
	const scanner = $derived(manifest?.annotations?.['io.angos.scan.scanner']);
	const scannedAt = $derived(manifest?.annotations?.['org.opencontainers.image.created']);

	$effect(() => {
		manifest = null;
		error = null;
		subjectTags = [];
		fetchManifest(data.path, data.digest).then(async (result) => {
			manifest = result.manifest;
			error = result.error;
			const scanned = result.manifest?.subject?.digest;
			if (!scanned) return;
			// The scanned manifest's tags, from the same listing the browse view reads.
			const revisions = await fetchRevisions(data.path);
			subjectTags = revisions.data?.manifests.find((m) => m.digest === scanned)?.tags ?? [];
		});
	});
</script>

<Breadcrumb
	items={[
		{ label: 'Repositories', href: `${base}/` },
		{ label: data.path, href: pathUrl(data.path) },
		{
			label: (subject ?? data.digest).slice(0, 19),
			href: manifestUrl(data.path, subject ?? data.digest)
		},
		{ label: 'Vulnerability report' }
	]}
/>

{#if error}
	<ErrorState message={error} />
{:else if manifest}
	<Card title="Report">
		<table>
			<tbody>
				{#if subject}
					<tr>
						<td class="label">Scanned manifest</td>
						<td>
							<DigestLink digest={subject} href={manifestUrl(data.path, subject)} />
							<TagList tags={subjectTags} getHref={(tag) => manifestUrl(data.path, tag)} />
						</td>
					</tr>
				{/if}
				<tr>
					<td class="label">Report manifest</td>
					<td><DigestLink digest={data.digest} href={manifestUrl(data.path, data.digest)} /></td>
				</tr>
				{#if scanner || scannedAt}
					<tr>
						<td class="label">Scanned</td>
						<td>
							{#if scannedAt}{formatTimeAgo(scannedAt)}{/if}
							{#if scanner}<span class="scan-meta">by {scanner}</span>{/if}
						</td>
					</tr>
				{/if}
			</tbody>
		</table>
	</Card>
	<ScanReport namespace={data.path} {manifest} />
{:else}
	<LoadingState message="Loading the report" />
{/if}
