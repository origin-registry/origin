<script lang="ts">
	import { parseScanSummary, SEVERITIES } from '$lib/utils';

	interface Props {
		annotations?: Record<string, string>;
	}

	let { annotations }: Props = $props();
	const summary = $derived(parseScanSummary(annotations));
</script>

{#if summary}
	<span class="scan-summary" title={summary.scanner ? `Scanned by ${summary.scanner}` : undefined}>
		{#if summary.total === 0}
			<span class="severity severity-clean">no findings</span>
		{:else}
			{#each SEVERITIES as severity}
				{#if summary.counts[severity] > 0}
					<span class="severity severity-{severity}">{summary.counts[severity]} {severity}</span>
				{/if}
			{/each}
		{/if}
	</span>
{/if}
