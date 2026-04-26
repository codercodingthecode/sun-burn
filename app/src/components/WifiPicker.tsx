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
      setNetworks(result)
    } catch (err) {
      console.error('WiFi scan failed:', err)
    } finally {
      setScanning(false)
    }
  })

  const selected = () => networks().find((n) => n.ssid === props.value)

  return (
    <div class="wifi-picker">
      <button
        type="button"
        class="wifi-trigger"
        onClick={() => setOpen((o) => !o)}
        disabled={scanning()}
      >
        <Show when={scanning()} fallback={
          <Show when={selected()} fallback={<span class="placeholder">Select network…</span>}>
            {(net) => (
              <>
                <SignalIcon bars={signalBars(net().signal_strength)} secured={net().secured} />
                <span>{net().ssid}</span>
              </>
            )}
          </Show>
        }>
          <span class="spinner" />
          <span>Scanning…</span>
        </Show>
        <span class="chevron">{open() ? '▲' : '▼'}</span>
      </button>

      <Show when={open() && !scanning()}>
        <div class="wifi-dropdown">
          <For each={networks()} fallback={<div class="wifi-empty">No networks found</div>}>
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
    </div>
  )
}

export default WifiPicker
