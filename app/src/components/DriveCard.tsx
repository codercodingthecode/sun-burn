import { Component } from 'solid-js'
import type { Drive } from '../types'

interface Props {
  drive: Drive
  selected: boolean
  onSelect: () => void
}

function formatBytes(bytes: number): string {
  const gb = bytes / 1e9
  if (gb >= 1) return `${gb.toFixed(0)} GB`
  const mb = bytes / 1e6
  return `${mb.toFixed(0)} MB`
}

const GB_64 = 64 * 1e9

const DriveCard: Component<Props> = (props) => {
  const large = () => props.drive.size_bytes > GB_64
  return (
    <button
      type="button"
      class={`drive-card ${props.selected ? 'drive-card--selected' : ''} ${large() ? 'drive-card--warn' : ''}`}
      onClick={props.onSelect}
    >
      <span class="drive-icon">
        {/* SD card SVG icon */}
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M6 2h9l3 3v16a1 1 0 01-1 1H6a1 1 0 01-1-1V3a1 1 0 011-1z" />
          <line x1="9" y1="2" x2="9" y2="7" />
          <line x1="12" y1="2" x2="12" y2="7" />
          <line x1="15" y1="5" x2="15" y2="7" />
        </svg>
      </span>
      <div class="drive-info">
        <span class="drive-name">{props.drive.display_name}</span>
        <span class="drive-path">{props.drive.path}</span>
      </div>
      <div class="drive-size-col">
        <span class={`drive-size ${large() ? 'drive-size--warn' : ''}`}>
          {formatBytes(props.drive.size_bytes)}
        </span>
        {large() && <span class="drive-warn-badge">Large</span>}
      </div>
      {props.selected && (
        <span class="drive-check">
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
            <circle cx="10" cy="10" r="9" stroke="#f59e0b" stroke-width="1.5" />
            <path d="M6 10l3 3 5-5" stroke="#f59e0b" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </span>
      )}
    </button>
  )
}

export default DriveCard
