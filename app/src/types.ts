export interface Manifest {
  version: string
  name: string
  description?: string
  steps: Step[]
}

export interface Step {
  id: string
  title: string
  fields: Field[]
  writes: WriteRule[]
}

export interface Field {
  id: string
  label: string
  type: 'text' | 'password' | 'wifi-picker' | 'ssh-key-picker' | 'country-picker' | 'timezone-picker' | 'toggle' | 'select'
  required: boolean
  default?: string
  show_when?: { field: string; value: string }
  options?: { value: string; label: string }[]
}

export interface WriteRule {
  path: string
  template: string
}

export interface WifiNetwork {
  ssid: string
  signal_strength: number
  secured: boolean
  frequency_ghz?: number
}

export interface Drive {
  path: string
  display_name: string
  size_bytes: number
  removable: boolean
}

export interface FlashProgress {
  bytes_written: number
  total_bytes: number
  speed_bps: number
}
