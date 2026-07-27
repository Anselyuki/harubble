<script lang="ts">
  import type { LogFileKind, LogFileStatus, LogViewerRecord } from '$lib/types';

  interface Props {
    logFileKind: LogFileKind;
    logRecords: LogViewerRecord[];
    logFileStatus: LogFileStatus | null;
    logViewerLoading: boolean;
    logViewerError: string;
    sectionTitle: string;
    logsDescription: string;
    logSegmentAria: string;
    logSessionLabel: string;
    logPersistentLabel: string;
    logStatusAvailableLabel: string;
    logStatusNoneLabel: string;
    logLoadingLabel: string;
    logEmptyLabel: string;
    onRefreshLogs: (kind: LogFileKind) => void;
  }

  let {
    logFileKind,
    logRecords,
    logFileStatus,
    logViewerLoading,
    logViewerError,
    sectionTitle,
    logsDescription,
    logSegmentAria,
    logSessionLabel,
    logPersistentLabel,
    logStatusAvailableLabel,
    logStatusNoneLabel,
    logLoadingLabel,
    logEmptyLabel,
    onRefreshLogs,
  }: Props = $props();
</script>

<section class="sheet-section settings-section">
  <div class="settings-section-heading settings-log-heading">
    <div>
      <h3>{sectionTitle}</h3>
      <p>{logsDescription}</p>
    </div>
    <div class="settings-segment" aria-label={logSegmentAria}>
      <button
        type="button"
        class:active={logFileKind === 'session'}
        aria-pressed={logFileKind === 'session'}
        onclick={() => onRefreshLogs('session')}>{logSessionLabel}</button
      >
      <button
        type="button"
        class:active={logFileKind === 'persistent'}
        aria-pressed={logFileKind === 'persistent'}
        onclick={() => onRefreshLogs('persistent')}>{logPersistentLabel}</button
      >
    </div>
  </div>
  <p class="settings-log-status">
    {logSessionLabel}: {logFileStatus?.hasSessionLog
      ? logStatusAvailableLabel
      : logStatusNoneLabel} · {logPersistentLabel}: {logFileStatus?.hasPersistentLog
      ? logStatusAvailableLabel
      : logStatusNoneLabel}
  </p>
  {#if logViewerLoading}
    <div class="settings-empty-state">{logLoadingLabel}</div>
  {:else if logViewerError}
    <div class="settings-error-state">{logViewerError}</div>
  {:else if logRecords.length > 0}
    <div class="settings-log-list">
      {#each logRecords as record (record.id)}
        <article class="settings-log-record">
          <div class="settings-log-meta">
            <span>{record.level}</span><time>{record.ts}</time>
          </div>
          <p class="settings-log-message">{record.message}</p>
          <p class="settings-log-source">
            {record.domain} · {record.code}
          </p>
          {#if record.details}<p class="settings-log-details">
              {record.details}
            </p>{/if}
        </article>
      {/each}
    </div>
  {:else}
    <div class="settings-empty-state">{logEmptyLabel}</div>
  {/if}
</section>

<style>
  .settings-log-heading {
    align-items: center;
  }
  .settings-log-status {
    margin: -4px 0 0;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .settings-log-list {
    display: grid;
    gap: 8px;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-md);
    background: var(--sheet-row-bg);
    padding: 8px;
  }
  .settings-log-record {
    display: grid;
    gap: 4px;
    border: 1px solid var(--sheet-border);
    border-radius: 7px;
    background: color-mix(in srgb, var(--bg-primary) 52%, transparent);
    padding: 8px 10px;
  }
  .settings-log-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.35;
  }
  .settings-log-meta span {
    font-weight: 700;
    text-transform: uppercase;
  }
  .settings-log-message {
    margin: 0;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.45;
  }
  .settings-log-source,
  .settings-log-details {
    margin: 0;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .settings-log-details {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .settings-empty-state,
  .settings-error-state {
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-md);
    background: var(--sheet-row-bg);
    padding: 14px 12px;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }
  .settings-error-state {
    border-color: color-mix(in srgb, var(--destructive) 40%, transparent);
    background: color-mix(in srgb, var(--destructive) 10%, transparent);
    color: var(--destructive);
  }
  @media (max-width: 420px) {
    .settings-log-heading {
      align-items: stretch;
    }
  }
</style>
