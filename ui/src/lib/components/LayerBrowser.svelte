<script lang="ts">
	import type { Descriptor, LayerListing } from '$lib/api';
	import { fetchLayerEntries, layerFileUrl } from '$lib/api';
	import { formatSize, mergeLayers, sortedChildren, type FsNode, type FsTree } from '$lib/utils';
	import Card from './Card.svelte';
	import FsRow, { type Matcher } from './FsRow.svelte';
	import LoadingState from './LoadingState.svelte';
	import ErrorState from './ErrorState.svelte';

	interface Props {
		namespace: string;
		/** The image's walkable layers, in order. */
		layers: Descriptor[];
	}

	let { namespace, layers }: Props = $props();

	/** Files up to this size open inline; larger ones only download. */
	const INLINE_LIMIT = 512 * 1024;

	let listings = $state<(LayerListing | null)[]>([]);
	let pending = $state(0);
	let error = $state<string | null>(null);
	let timer: ReturnType<typeof setTimeout> | undefined;

	let layerFilter = $state<number | null>(null);
	let search = $state('');
	let expanded = $state<Set<string>>(new Set());
	let selected = $state<FsNode | null>(null);
	let content = $state<{ text: string | null; binary: boolean; tooBig: boolean } | null>(null);
	let contentError = $state<string | null>(null);

	// Every layer is asked at once; the registry indexes the ones it never
	// saw and answers 202 until it has, so the load asks again shortly.
	async function load(pass: number) {
		const results = await Promise.all(
			layers.map((layer) => fetchLayerEntries(namespace, layer.digest))
		);
		listings = results.map((result) => result.listing);
		pending = results.filter((result) => result.pending).length;
		error = results.find((result) => result.error)?.error ?? null;
		if (pending > 0 && !error) timer = setTimeout(() => load(pass + 1), Math.min(2000 * (pass + 1), 10000));
	}

	$effect(() => {
		void namespace;
		void layers;
		listings = [];
		pending = 0;
		error = null;
		selected = null;
		content = null;
		expanded = new Set();
		layerFilter = null;
		search = '';
		clearTimeout(timer);
		load(0);
		return () => clearTimeout(timer);
	});

	const tree = $derived.by((): FsTree | null =>
		listings.length === layers.length && listings.every(Boolean)
			? mergeLayers(listings as LayerListing[])
			: null
	);
	const matcher = $derived<Matcher>({ layer: layerFilter, text: search.trim().toLowerCase() });
	const deleted = $derived(
		tree && layerFilter !== null ? tree.deletions.filter((d) => d.layer === layerFilter) : []
	);
	const total = $derived(listings.reduce((sum, listing) => sum + (listing?.entries.length ?? 0), 0));
	const layerlabel = (layer: number) => `L${layer + 1}`;
	/** The selected node when it has bytes to show. */
	const file = $derived(
		selected && (selected.kind === 'file' || selected.kind === 'hardlink') ? selected : null
	);
	const downloadUrl = $derived(
		file ? layerFileUrl(namespace, layers[file.layer].digest, file.path, true) : ''
	);

	function toggle(path: string) {
		const next = new Set(expanded);
		if (!next.delete(path)) next.add(path);
		expanded = next;
	}

	async function open(node: FsNode) {
		selected = node;
		content = null;
		contentError = null;
		if (node.kind !== 'file' && node.kind !== 'hardlink') return;
		if ((node.entry?.size ?? 0) > INLINE_LIMIT) {
			content = { text: null, binary: false, tooBig: true };
			return;
		}
		try {
			const response = await fetch(layerFileUrl(namespace, layers[node.layer].digest, node.path));
			if (!response.ok) {
				contentError = `HTTP ${response.status}`;
				return;
			}
			const bytes = new Uint8Array(await response.arrayBuffer());
			const binary = bytes.subarray(0, 8192).some((byte) => byte === 0);
			content = { text: binary ? null : new TextDecoder().decode(bytes), binary, tooBig: false };
		} catch (e) {
			contentError = e instanceof Error ? e.message : 'Request failed';
		}
	}
</script>

<Card title="Filesystem" count={tree ? total : undefined}>
	{#if error}
		<ErrorState message="Could not load the layer listings ({error})." />
	{:else if !tree}
		<LoadingState
			message={pending > 0
				? `Indexing layer ${layers.length - pending + 1} of ${layers.length}, this takes a moment the first time`
				: 'Loading the layer listings'}
		/>
	{:else}
		<div class="fs-toolbar">
			<select bind:value={layerFilter} aria-label="Layer">
				<option value={null}>All layers</option>
				{#each layers as layer, i}
					<option value={i}>{layerlabel(i)} · {formatSize(layer.size)} · {layer.digest.slice(7, 19)}</option>
				{/each}
			</select>
			<input type="search" placeholder="Filter paths" bind:value={search} aria-label="Filter paths" />
		</div>
		<table>
			<thead>
				<tr>
					<th>Name</th>
					<th class="col-narrow">Size</th>
					<th class="col-medium">Mode</th>
					<th class="col-medium">Modified</th>
					<th class="col-narrow">Layer</th>
				</tr>
			</thead>
			<tbody>
				{#each sortedChildren(tree.root) as node (node.path)}
					<FsRow
						{node}
						depth={0}
						{expanded}
						{matcher}
						selected={selected?.path ?? null}
						ontoggle={toggle}
						onopen={open}
						{layerlabel}
					/>
				{/each}
				{#each deleted as gone}
					<tr>
						<td class="fs-name fs-deleted" colspan="4"><span class="fs-toggle"></span>{gone.path}</td>
						<td><span class="badge">{layerlabel(gone.layer)}</span></td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</Card>

{#if file}
	{#snippet actions()}
		<a class="btn secondary" href={downloadUrl} download={file.name}>Download</a>
	{/snippet}
	<Card title={file.path} headerActions={actions}>
		<div class="fs-content">
			{#if contentError}
				<ErrorState message="Could not read the file ({contentError})." />
			{:else if !content}
				<LoadingState message="Reading the file" />
			{:else if content.tooBig}
				<p class="muted">{formatSize(file.entry?.size ?? 0)}, too large to show here.</p>
			{:else if content.binary}
				<p class="muted">Binary file, {formatSize(file.entry?.size ?? 0)}.</p>
			{:else}
				<pre>{content.text}</pre>
			{/if}
		</div>
	</Card>
{/if}
