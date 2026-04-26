import { Component, createSignal, createEffect, For, Show } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import type { WifiNetwork } from '../types'

interface Props {
  value: string
  onChange: (ssid: string) => void
}

function signalBars(strength: number): number {
  if (strength >= -55) return 4
  if (strength >= -65) return 3
  if (strength >= -75) return 2
  return 1
}

function SignalIcon(props: { bars: number; secured: boolean }) {
  return (
    <span class="wifi-signal" title={`${props.bars}/4 bars${props.secured ? ', secured' : ''}`}>
      {[1, 2, 3, 4].map((b) => (
        <span class={`wifi-bar ${props.bars >= b ? 'wifi-bar--active' : ''}`} style={{ height: `${b * 3 + 3}px` }} />
      ))}
      {props.secured && <span class="wifi-lock">&#128274;</span>}
    </span>
  )
}

const WifiPicker: Component<Props> = (props) => {
  const [networks, setNetworks] = createSignal<WifiNetwork[]>([])
  const [scanning, setScanning] = createSignal(true)
  const [open, setOpen] = createSignal(false)

  createEffect(async () => {
    setScanning(true)
    try {
      const result = await invoke<WifiNetwork[]>('scan_wifi_networks')
      // Filter out blank or redacted SSIDs (macOS returns these without Location permission)
      const valid = result.filter(n => n.ssid && n.ssid !== '<redacted>' && n.ssid.trim() !== '')
      setNetworks(valid)
    } catch (err) {
      console.error('WiFi scan failed:', err)
    } finally {
      setScanning(false)
    }
  })

  const visibleNetworks = () => networks()
  const hasNetworks = () => visibleNetworks().length > 0

  return (
    <div class="wifi-picker">
      {/* Always-visible text input */}
      <input
        type="text"
        class="wifi-text-input"
        placeholder="Enter network name…"
        value={props.value}
        onInput={(e) => props.onChange(e.currentTarget.value)}
      />

      {/* Scan dropdown — shown when we have real results */}
      <Show when={!scanning() && hasNetworks()}>
        <button
          type="button"
          class="wifi-scan-toggle"
          onClick={() => setOpen((o) => !o)}
        >
          {open() ? '▲ Hide nearby networks' : `▼ Nearby networks (${visibleNetworks().length})`}
        </button>

        <Show when={open()}>
          <div class="wifi-dropdown">
            <For each={visibleNetworks()}>
              {(net) => (
                <button
                  type="button"
                  class={`wifi-option ${net.ssid === props.value ? 'wifi-option--selected' : ''}`}
                  onClick={() => { props.onChange(net.ssid); setOpen(false) }}
                >
                  <SignalIcon bars={signalBars(net.signal_strength)} secured={net.secured} />
                  <span class="wifi-ssid">{net.ssid}</span>
                  <span class="wifi-meta">{net.signal_strength} dBm{net.frequency_ghz ? ` · ${net.frequency_ghz} GHz` : ''}</span>
                </button>
              )}
            </For>
          </div>
        </Show>
      </Show>

      <Show when={scanning()}>
        <span class="wifi-scanning-hint"><span class="spinner" /> Scanning for networks…</span>
      </Show>
    </div>
  )
}

export default WifiPicker
