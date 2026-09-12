<script lang="ts">
	import { formatMode, formatSize, sortedChildren, type FsNode } from '$lib/utils';
	import FsRow from './FsRow.svelte';

	/** What the tree is narrowed to: one layer's changes, a path fragment, or nothing. */
	export interface Matcher {
		layer: number | null;
		text: string;
	}

	interface Props {
		node: FsNode;
		depth: number;
		expanded: Set<string>;
		matcher: Matcher;
		selected: string | null;
		ontoggle: (path: string) => void;
		onopen: (node: FsNode) => void;
		layerlabel: (layer: number) => string;
	}

	let { node, depth, expanded, matcher, selected, ontoggle, onopen, layerlabel }: Props = $props();

	const filtering = $derived(matcher.layer !== null || matcher.text !== '');
	function matches(n: FsNode): boolean {
		return (
			(matcher.layer === null || n.layer === matcher.layer) &&
			(matcher.text === '' || n.path.toLowerCase().includes(matcher.text))
		);
	}
	/** A node stays in a narrowed tree when it or anything under it matches. */
	function visible(n: FsNode): boolean {
		return matches(n) || [...n.children.values()].some(visible);
	}
	const shown = $derived(!filtering || visible(node));
	// A narrowed tree opens itself: the matches are what the reader asked for.
	const open = $derived(node.kind === 'dir' && (filtering || expanded.has(node.path)));
	const children = $derived(open ? sortedChildren(node) : []);
	const modified = $derived(
		node.entry ? new Date(node.entry.mtime * 1000).toISOString().slice(0, 10) : ''
	);
</script>

{#if shown}
	<tr
		class="clickable"
		class:dim={filtering && !matches(node)}
		class:selected={selected === node.path}
		onclick={() => (node.kind === 'dir' ? ontoggle(node.path) : onopen(node))}
	>
		<td class="fs-name" style="padding-left: calc({depth} * 1.25rem + 0.75rem)">
			{#if node.kind === 'dir'}
				<span class="fs-toggle" aria-hidden="true">{open ? '▾' : '▸'}</span>
			{:else}
				<span class="fs-toggle" aria-hidden="true"></span>
			{/if}
			<span>{node.name}</span>
			{#if node.entry?.link}
				<span class="fs-link">→ {node.entry.link}</span>
			{/if}
		</td>
		<td class="nowrap">{node.kind === 'file' ? formatSize(node.entry?.size ?? 0) : ''}</td>
		<td class="mono">{node.entry ? formatMode(node.entry.mode) : ''}</td>
		<td class="nowrap">{modified}</td>
		<td><span class="badge">{layerlabel(node.layer)}</span></td>
	</tr>
	{#each children as child (child.path)}
		<FsRow node={child} depth={depth + 1} {expanded} {matcher} {selected} {ontoggle} {onopen} {layerlabel} />
	{/each}
{/if}

<style>
	.dim {
		color: var(--muted);
	}
	.selected td {
		background: var(--accent-soft);
	}
</style>
