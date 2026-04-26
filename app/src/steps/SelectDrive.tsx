import { Component, createSignal, createEffect, For, Show } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { state, setState, goNext, goBack } from '../store'
import type { Drive } from '../types'
import { mockDrives } from '../mock'
import DriveCard from '../components/DriveCard'

const SelectDrive: Component = () => {
  const [drives, setDrives] = createSignal<Drive[]>([])
  const [loading, setLoading] = createSignal(true)
  const [refreshing, setRefreshing] = createSignal(false)

  async function loadDrives(isRefresh = false) {
    if (isRefresh) setRefreshing(true)
    else setLoading(true)
    try {
      const result = await invoke<Drive[]>('list_removable_drives')
      setDrives(result)
    } catch {
      setDrives(mockDrives)
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }

  createEffect(() => { void loadDrives() })

  return (
    <div class="step-content">
      <div class="step-header">
        <h1 class="step-title">Select Drive</h1>
        <p class="step-subtitle">Choose the target SD card or USB drive. All existing data will be overwritten.</p>
      </div>

      <Show when={!loading()} fallback={
        <div class="drive-loading">
          <span class="spinner spinner--lg" />
          <span>Scanning for drives…</span>
        </div>
      }>
        <div class="drive-list">
          <For each={drives()} fallback={
            <div class="drive-empty">
              <svg width="40" height="40" viewBox="0 0 40 40" fill="none" stroke="#71717a" stroke-width="1.2">
                <rect x="8" y="6" width="24" height="28" rx="3" />
                <path d="M14 12h12M14 18h8" stroke-linecap="round" />
              </svg>
              <p>No removable drives found</p>
              <p class="drive-empty-hint">Insert an SD card or USB drive</p>
            </div>
          }>
            {(drive) => (
              <DriveCard
                drive={drive}
                selected={state.selectedDrive?.path === drive.path}
                onSelect={() => setState('selectedDrive', drive)}
              />
            )}
          </For>
        </div>

        {drives().some((d) => d.size_bytes > 64e9) && (
          <div class="drive-warn-banner">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M8 2L15 14H1L8 2z" stroke="#ef4444" stroke-width="1.3" stroke-linejoin="round" />
              <path d="M8 6v4M8 11v1" stroke="#ef4444" stroke-width="1.4" stroke-linecap="round" />
            </svg>
            <span>Drives larger than 64 GB are unusual for SD cards — double-check your selection.</span>
          </div>
        )}
      </Show>

      <div class="drive-toolbar">
        <button type="button" class="btn btn--ghost btn--sm" onClick={() => loadDrives(true)} disabled={refreshing()}>
          {refreshing() ? <span class="spinner" /> : '↺'} Refresh
        </button>
      </div>

      <div class="step-actions">
        <button type="button" class="btn btn--ghost" onClick={goBack}>← Back</button>
        <button
          type="button"
          class="btn btn--primary"
          disabled={!state.selectedDrive}
          onClick={goNext}
        >
          Continue →
        </button>
      </div>
    </div>
  )
}

export default SelectDrive
