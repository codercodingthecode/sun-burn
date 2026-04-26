import { Component, createSignal } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { setState, goNext, state } from '../store'
import type { Manifest } from '../types'
import { mockManifest } from '../mock'

const SelectImage: Component = () => {
  const [loading, setLoading] = createSignal(false)
  const [localError, setLocalError] = createSignal<string | null>(null)

  async function pickFile() {
    setLocalError(null)
    setLoading(true)

    try {
      // Try Tauri open_file_dialog command (registered in backend) or fall back to mock
      let filePath: string | null = null
      try {
        filePath = await invoke<string | null>('open_file_dialog')
      } catch {
        // Backend not running or dialog not exposed — use a mock path for development
        filePath = '/mock/solar-monitor.img'
      }

      if (!filePath) {
        setLoading(false)
        return
      }

      setState('imagePath', filePath)

      // Try to read the manifest
      let manifest: Manifest | null = null
      try {
        manifest = await invoke<Manifest | null>('read_manifest', { imagePath: filePath })
      } catch {
        // Backend not running — use mock manifest for development
        manifest = mockManifest
      }

      setState('manifest', manifest)

      // Pre-populate defaults
      if (manifest) {
        for (const step of manifest.steps) {
          for (const field of step.fields) {
            if (field.default !== undefined && !(field.id in state.fieldValues)) {
              setState('fieldValues', field.id, field.default)
            }
          }
        }
      }

      goNext()
    } catch (err) {
      setLocalError(String(err))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div class="step-content step-select-image">
      <div class="step-header">
        <h1 class="step-title">Select Image</h1>
        <p class="step-subtitle">
          Choose a <code class="code-inline">.img</code> or <code class="code-inline">.iso</code> file
          to flash. If it contains a <code class="code-inline">sunburn.json</code> manifest, you'll be
          guided through configuration first.
        </p>
      </div>

      <div
        class="image-drop-area"
        onClick={pickFile}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === 'Enter' && pickFile()}
      >
        {loading() ? (
          <div class="drop-loading">
            <span class="spinner spinner--lg" />
            <span>Reading image…</span>
          </div>
        ) : (
          <>
            <div class="drop-icon">
              <svg width="48" height="48" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.5">
                <rect x="8" y="6" width="32" height="36" rx="3" />
                <path d="M28 6v10h10" />
                <path d="M20 28l4-4 4 4M24 24v10" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            </div>
            <p class="drop-label">Click to select image file</p>
            <p class="drop-formats">.img · .iso · .zip · .gz · .xz</p>
          </>
        )}
      </div>

      {localError() && <div class="field-error">{localError()}</div>}

      <div class="step-hint">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <circle cx="8" cy="8" r="7" stroke="#71717a" stroke-width="1.2" />
          <path d="M8 7v5M8 5v.5" stroke="#71717a" stroke-width="1.4" stroke-linecap="round" />
        </svg>
        <span>
          Images with a <code class="code-inline">sunburn.json</code> manifest will show a
          configuration wizard before flashing
        </span>
      </div>
    </div>
  )
}

export default SelectImage
