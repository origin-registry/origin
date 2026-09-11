<script lang="ts">
	import { goto } from '$app/navigation';
	import { formatTimeAgo, parseScanSummary, SEVERITIES } from '$lib/utils';
	import Card from './Card.svelte';

	interface Props {
		/** The report manifest's annotations, carrying the `io.angos.scan.*` summary. */
		annotations?: Record<string, string>;
		/** The report page. */
		href: string;
	}

	let { annotations, href }: Props = $props();
	const summary = $derived(parseScanSummary(annotations));
	const scannedAt = $derived(annotations?.['org.opencontainers.image.created']);
	const rows = $derived(SEVERITIES.filter((severity) => (summary?.counts[severity] ?? 0) > 0));
</script>

{#snippet meta()}
	<span class="scan-meta">
		{#if scannedAt}{formatTimeAgo(scannedAt)}{/if}
		{#if summary?.scanner}by {summary.scanner}{/if}
	</span>
{/snippet}

{#if summary}
	<div class="scan-card">
		<Card title="Vulnerabilities" count={summary.total} headerActions={meta}>
			<table>
				<thead>
					<tr>
						<th>Severity</th>
						<th>Findings</th>
					</tr>
				</thead>
				<tbody>
					{#if rows.length === 0}
						<tr class="clickable" onclick={() => goto(href)}>
							<td><span class="severity severity-clean">none</span></td>
							<td>0</td>
						</tr>
					{:else}
						{#each rows as severity}
							<tr class="clickable" onclick={() => goto(href)}>
								<td><span class="severity severity-{severity}">{severity}</span></td>
								<td>{summary.counts[severity]}</td>
							</tr>
						{/each}
					{/if}
				</tbody>
			</table>
		</Card>
	</div>
{/if}
