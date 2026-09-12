<script lang="ts">
	import { goto } from '$app/navigation';
	import { formatTimeAgo, parseScanSummary, SEVERITIES } from '$lib/utils';
	import Card from './Card.svelte';

	/** A report to summarise; `label` names its tab when there are several. */
	interface Report {
		label?: string;
		/** The report manifest's annotations, carrying the `io.angos.scan.*` summary. */
		annotations?: Record<string, string>;
		/** The report page. */
		href: string;
	}

	let { reports }: { reports: Report[] } = $props();

	const parsed = $derived(
		reports.flatMap((report) => {
			const summary = parseScanSummary(report.annotations);
			return summary ? [{ ...report, summary }] : [];
		})
	);
	// The tab shown; an index has one per platform manifest with a report.
	let selected = $state(0);
	const current = $derived(parsed[Math.min(selected, parsed.length - 1)]);
	const scannedAt = $derived(current?.annotations?.['org.opencontainers.image.created']);
	const rows = $derived(SEVERITIES.filter((severity) => (current?.summary.counts[severity] ?? 0) > 0));
</script>

{#snippet header()}
	<span class="scan-meta">
		{#if scannedAt}{formatTimeAgo(scannedAt)}{/if}
		{#if current?.summary.scanner}by {current.summary.scanner}{/if}
	</span>
{/snippet}

{#if current}
	<div class="scan-card">
		<Card title="Vulnerabilities" count={current.summary.total} headerActions={header}>
			{#if parsed.length > 1}
				<div class="card-tabs">
					<div class="view-toggle" role="tablist" aria-label="Platform">
						{#each parsed as report, i (report.href)}
							<button role="tab" aria-selected={i === selected} class:active={i === selected} onclick={() => (selected = i)}>
								{report.label}
							</button>
						{/each}
					</div>
				</div>
			{/if}
			<table>
				<thead>
					<tr>
						<th>Severity</th>
						<th>Findings</th>
					</tr>
				</thead>
				<tbody>
					{#if rows.length === 0}
						<tr class="clickable" onclick={() => goto(current.href)}>
							<td><span class="severity severity-clean">none</span></td>
							<td>0</td>
						</tr>
					{:else}
						{#each rows as severity (severity)}
							<tr class="clickable" onclick={() => goto(current.href)}>
								<td><span class="severity severity-{severity}">{severity}</span></td>
								<td>{current.summary.counts[severity]}</td>
							</tr>
						{/each}
					{/if}
				</tbody>
			</table>
		</Card>
	</div>
{/if}
