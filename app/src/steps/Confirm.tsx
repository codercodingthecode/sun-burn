import { Component, For, Show } from 'solid-js'
import { state, goNext, goBack } from '../store'

function formatBytes(bytes: number): string {
  const gb = bytes / 1e9
  if (gb >= 1) return `${gb.toFixed(1)} GB`
  return `${(bytes / 1e6).toFixed(0)} MB`
}

function baseName(path: string): string {
  return path.split('/').pop() ?? path
}

const Confirm: Component = () => {
  const manifest = () => state.manifest
  const drive = () => state.selectedDrive!

  // Collect all field values that are set
  const configuredFields = () => {
    const mf = manifest()
    if (!mf) return []
    const rows: { label: string; value: string }[] = []
    for (const step of mf.steps) {
      for (const field of step.fields) {
        const val = state.fieldValues[field.id]
        if (val !== undefined && val !== '') {
          // Mask passwords
          const display = field.type === 'password' ? '••••••••' : val
          rows.push({ label: field.label, value: display })
        }
      }
    }
    return rows
  }

  return (
    <div class="step-content">
      <div class="step-header">
        <h1 class="step-title">Ready to Flash</h1>
        <p class="step-subtitle">Review your configuration. This will permanently overwrite the selected drive.</p>
      </div>

      <div class="confirm-sections">
        {/* Image */}
        <div class="confirm-card">
          <div class="confirm-card-header">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
              <rect x="2" y="1" width="12" height="14" rx="2" />
              <path d="M5 5h6M5 8h4" stroke-linecap="round" />
              <path d="M7 11l1.5-1.5L11 12" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            <span>Image</span>
          </div>
          <div class="confirm-card-body">
            <span class="confirm-value">{baseName(state.imagePath ?? '')}</span>
            <Show when={manifest()}>
              {(mf) => <span class="confirm-sub">{mf().name} · v{mf().version}</span>}
            </Show>
          </div>
        </div>

        {/* Drive */}
        <div class="confirm-card">
          <div class="confirm-card-header">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
              <rect x="2" y="2" width="12" height="12" rx="2" />
              <path d="M5 6h6M5 9h4" stroke-linecap="round" />
            </svg>
            <span>Drive</span>
          </div>
          <div class="confirm-card-body">
            <span class="confirm-value">{drive().display_name}</span>
            <span class="confirm-sub">{drive().path} · {formatBytes(drive().size_bytes)}</span>
          </div>
        </div>

        {/* Config summary */}
        <Show when={configuredFields().length > 0}>
          <div class="confirm-card confirm-card--config">
            <div class="confirm-card-header">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
                <circle cx="8" cy="8" r="2.5" />
                <path d="M8 1v2M8 13v2M1 8h2M13 8h2M3.05 3.05l1.41 1.41M11.54 11.54l1.41 1.41M3.05 12.95l1.41-1.41M11.54 4.46l1.41-1.41" stroke-linecap="round" />
              </svg>
              <span>Configuration</span>
            </div>
            <div class="confirm-fields">
              <For each={configuredFields()}>
                {(row) => (
                  <div class="confirm-field-row">
                    <span class="confirm-field-label">{row.label}</span>
                    <span class="confirm-field-value">{row.value}</span>
                  </div>
                )}
              </For>
            </div>
          </div>
        </Show>
      </div>

      <div class="confirm-warning">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M8 2L15 14H1L8 2z" stroke="#f59e0b" stroke-width="1.3" stroke-linejoin="round" />
          <path d="M8 6v4M8 11v1" stroke="#f59e0b" stroke-width="1.4" stroke-linecap="round" />
        </svg>
        <span>
          Flashing will <strong>erase all data</strong> on <strong>{drive().display_name}</strong>. This cannot be undone.
        </span>
      </div>

      <div class="step-actions">
        <button type="button" class="btn btn--ghost" onClick={goBack}>← Back</button>
        <button type="button" class="btn btn--flash" onClick={goNext}>
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path d="M10 2L4 10h5l-1 6 7-8H10l1-6z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
          </svg>
          Flash Image
        </button>
      </div>
    </div>
  )
}

export default Confirm
