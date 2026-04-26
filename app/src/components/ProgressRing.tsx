import { Component } from 'solid-js'

interface Props {
  progress: number // 0–1
  size?: number
  strokeWidth?: number
  label?: string
}

const ProgressRing: Component<Props> = (props) => {
  const size = () => props.size ?? 200
  const sw = () => props.strokeWidth ?? 10
  const r = () => (size() - sw()) / 2
  const cx = () => size() / 2
  const circumference = () => 2 * Math.PI * r()
  const dash = () => circumference() * Math.min(1, Math.max(0, props.progress))
  const gap = () => circumference() - dash()
  const pct = () => Math.round(props.progress * 100)

  return (
    <div class="progress-ring-wrap" style={{ width: `${size()}px`, height: `${size()}px`, position: 'relative' }}>
      <svg width={size()} height={size()} style={{ transform: 'rotate(-90deg)' }}>
        {/* Track */}
        <circle
          cx={cx()}
          cy={cx()}
          r={r()}
          fill="none"
          stroke="#2a2a2e"
          stroke-width={sw()}
        />
        {/* Progress arc */}
        <circle
          cx={cx()}
          cy={cx()}
          r={r()}
          fill="none"
          stroke="#f59e0b"
          stroke-width={sw()}
          stroke-linecap="round"
          stroke-dasharray={`${dash()} ${gap()}`}
          style={{ transition: 'stroke-dasharray 0.3s ease' }}
        />
      </svg>
      <div
        style={{
          position: 'absolute',
          inset: '0',
          display: 'flex',
          'flex-direction': 'column',
          'align-items': 'center',
          'justify-content': 'center',
          gap: '2px',
        }}
      >
        <span style={{ 'font-size': '2.5rem', 'font-weight': '700', color: '#f4f4f5', 'line-height': '1' }}>
          {pct()}%
        </span>
        {props.label && (
          <span style={{ 'font-size': '0.75rem', color: '#71717a' }}>{props.label}</span>
        )}
      </div>
    </div>
  )
}

export default ProgressRing
