<script lang="ts">
	interface Props {
		text: string;
		/** The accessible name, also the tooltip. */
		label: string;
	}

	let { text, label }: Props = $props();
	let copied = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	async function copy() {
		try {
			await navigator.clipboard.writeText(text);
		} catch {
			// No clipboard access: the button simply does nothing.
			return;
		}
		copied = true;
		clearTimeout(timer);
		timer = setTimeout(() => (copied = false), 1500);
	}
</script>

<button type="button" class="copy" class:copied onclick={copy} title={copied ? 'Copied' : label} aria-label={label}>
	{#if copied}
		<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 5 5L20 7" /></svg>
	{:else}
		<svg viewBox="0 0 24 24" aria-hidden="true">
			<rect x="9" y="9" width="12" height="12" rx="2" />
			<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
		</svg>
	{/if}
</button>

<style>
	.copy {
		flex: none;
		padding: 0.25rem;
		border-radius: var(--radius-sm);
		background: none;
		box-shadow: none;
		color: var(--muted);
		vertical-align: middle;
	}
	.copy:hover {
		background: var(--hover);
		color: var(--text);
	}
	.copy.copied,
	.copy.copied:hover {
		color: var(--ok-fg);
	}
	svg {
		width: 1rem;
		height: 1rem;
		fill: none;
		stroke: currentColor;
		stroke-width: 2;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
