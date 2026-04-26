import { Component, createEffect, onCleanup, For } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { state, setState, goNext } from '../store'
import type { FlashProgress } from '../types'
import ProgressRing from '../components/ProgressRing'

function formatBytes(bytes: number): string {
  const gb = bytes / 1e9
  if (gb >= 0.1) return `${gb.toFixed(2)} GB`
  return `${(bytes / 1e6).toFixed(0)} MB`
}

function formatSpeed(bps: number): string {
  const mbps = bps / 1e6
  if (mbps >= 1) return `${mbps.toFixed(1)} MB/s`
  return `${(bps / 1e3).toFixed(0)} KB/s`
}

function formatEta(written: number, total: number, bps: number): string {
  if (bps <= 0 || written >= total) return ''
  const remaining = total - written
  const secs = remaining / bps
  if (secs < 60) return `~${Math.round(secs)}s remaining`
  return `~${Math.round(secs / 60)}m remaining`
}

const Flashing: Component = () => {
  const progress = () => state.flashProgress
  const log = () => state.flashLog

  const pct = () => {
    const p = progress()
    if (!p || p.total_bytes === 0) return 0
    return p.bytes_written / p.total_bytes
  }

  const label = () => {
    const p = progress()
    if (!p) return 'Preparing…'
    if (p.bytes_written >= p.total_bytes) return 'Done!'
    return formatSpeed(p.speed_bps)
  }

  function addLog(msg: string) {
    setState('flashLog', (l) => [...l, msg])
  }

  createEffect(async () => {
    setState('flashLog', [])
    setState('flashProgress', null)
    setState('error', null)

    try {
      addLog('Preparing image…')

      if (state.manifest && Object.keys(state.fieldValues).length > 0) {
        addLog('Applying configuration to boot partition…')
        await invoke('patch_image', {
          imagePath: state.imagePath,
          values: state.fieldValues,
        })
        addLog('Configuration written.')
      }

      addLog(`Staging image and unmounting ${state.selectedDrive?.path}…`)
      addLog('Requesting admin password to write to disk…')

      const unlisten = await listen<FlashProgress>('flash-progress', (event) => {
        setState('flashProgress', event.payload)
      })

      try {
        await invoke('flash_image', {
          imagePath: state.imagePath,
          drivePath: state.selectedDrive?.path,
        })
      } finally {
        unlisten()
      }

      addLog('Finalizing…')
      addLog('Done!')
      await new Promise((r) => setTimeout(r, 800))
      goNext()
    } catch (err) {
      const msg = String(err)
      addLog(`Error: ${msg}`)
      setState('error', msg)
    }
  })

  return (
    <div class="step-content step-flashing">
      <div class="step-header">
        <h1 class="step-title">Flashing…</h1>
        <p class="step-subtitle">Do not remove the drive until the process completes.</p>
      </div>

      <div class="flash-ring-wrap">
        <ProgressRing progress={pct()} size={200} strokeWidth={12} label={label()} />
      </div>

      <div class="flash-stats">
        {progress() && (
          <>
            <span class="flash-stat">
              {formatBytes(progress()!.bytes_written)} / {formatBytes(progress()!.total_bytes)}
            </span>
            <span class="flash-stat-sep">·</span>
            <span class="flash-stat">{formatSpeed(progress()!.speed_bps)}</span>
            <span class="flash-stat-sep">·</span>
            <span class="flash-stat">{formatEta(progress()!.bytes_written, progress()!.total_bytes, progress()!.speed_bps)}</span>
          </>
        )}
      </div>

      <div class="flash-log">
        <For each={log()}>
          {(line) => <div class="flash-log-line">{line}</div>}
        </For>
      </div>
    </div>
  )
}

export default Flashing
