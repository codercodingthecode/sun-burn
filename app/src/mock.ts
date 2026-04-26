import type { Manifest, WifiNetwork, Drive } from './types'

export const mockManifest: Manifest = {
  version: '1',
  name: 'Solar Monitor',
  description: 'Home solar monitoring stack',
  steps: [
    {
      id: 'network',
      title: 'Network Setup',
      fields: [
        { id: 'ssid', label: 'WiFi Network', type: 'wifi-picker', required: true },
        { id: 'password', label: 'Password', type: 'password', required: true },
        { id: 'country', label: 'Country', type: 'country-picker', required: false, default: 'US' },
      ],
      writes: [],
    },
    {
      id: 'device',
      title: 'Device',
      fields: [
        { id: 'hostname', label: 'Device name', type: 'text', required: true, default: 'my-device' },
        { id: 'timezone', label: 'Timezone', type: 'timezone-picker', required: false, default: 'auto' },
      ],
      writes: [],
    },
    {
      id: 'access',
      title: 'Access',
      fields: [
        { id: 'ssh_enabled', label: 'Enable SSH', type: 'toggle', required: false, default: 'false' },
        {
          id: 'ssh_key',
          label: 'SSH Public Key',
          type: 'ssh-key-picker',
          required: false,
          show_when: { field: 'ssh_enabled', value: 'true' },
        },
      ],
      writes: [],
    },
  ],
}

export const mockNetworks: WifiNetwork[] = [
  { ssid: 'HomeNetwork', signal_strength: -45, secured: true, frequency_ghz: 5 },
  { ssid: 'Neighbor_5G', signal_strength: -72, secured: true, frequency_ghz: 5 },
  { ssid: 'OpenCafe', signal_strength: -60, secured: false, frequency_ghz: 2.4 },
  { ssid: 'AndroidAP', signal_strength: -80, secured: true, frequency_ghz: 2.4 },
]

export const mockDrives: Drive[] = [
  {
    path: '/dev/disk4',
    display_name: 'SanDisk Ultra (32 GB)',
    size_bytes: 32010928128,
    removable: true,
  },
  {
    path: '/dev/disk5',
    display_name: 'Generic Storage (16 GB)',
    size_bytes: 16005931008,
    removable: true,
  },
]

export const COUNTRIES = [
  { value: 'AU', label: 'Australia' },
  { value: 'BR', label: 'Brazil' },
  { value: 'CA', label: 'Canada' },
  { value: 'CN', label: 'China' },
  { value: 'DE', label: 'Germany' },
  { value: 'FR', label: 'France' },
  { value: 'GB', label: 'United Kingdom' },
  { value: 'IN', label: 'India' },
  { value: 'JP', label: 'Japan' },
  { value: 'NL', label: 'Netherlands' },
  { value: 'NZ', label: 'New Zealand' },
  { value: 'SE', label: 'Sweden' },
  { value: 'US', label: 'United States' },
  { value: 'ZA', label: 'South Africa' },
]

export const TIMEZONES = [
  { value: 'auto', label: 'Auto-detect' },
  { value: 'America/New_York', label: 'America/New_York (ET)' },
  { value: 'America/Chicago', label: 'America/Chicago (CT)' },
  { value: 'America/Denver', label: 'America/Denver (MT)' },
  { value: 'America/Los_Angeles', label: 'America/Los_Angeles (PT)' },
  { value: 'America/Anchorage', label: 'America/Anchorage (AKT)' },
  { value: 'Pacific/Honolulu', label: 'Pacific/Honolulu (HT)' },
  { value: 'America/Toronto', label: 'America/Toronto' },
  { value: 'America/Vancouver', label: 'America/Vancouver' },
  { value: 'America/Sao_Paulo', label: 'America/Sao_Paulo' },
  { value: 'Europe/London', label: 'Europe/London (GMT)' },
  { value: 'Europe/Paris', label: 'Europe/Paris (CET)' },
  { value: 'Europe/Berlin', label: 'Europe/Berlin (CET)' },
  { value: 'Europe/Stockholm', label: 'Europe/Stockholm' },
  { value: 'Europe/Amsterdam', label: 'Europe/Amsterdam' },
  { value: 'Asia/Kolkata', label: 'Asia/Kolkata (IST)' },
  { value: 'Asia/Tokyo', label: 'Asia/Tokyo (JST)' },
  { value: 'Asia/Shanghai', label: 'Asia/Shanghai (CST)' },
  { value: 'Asia/Singapore', label: 'Asia/Singapore' },
  { value: 'Australia/Sydney', label: 'Australia/Sydney (AEST)' },
  { value: 'Australia/Melbourne', label: 'Australia/Melbourne' },
  { value: 'Pacific/Auckland', label: 'Pacific/Auckland (NZST)' },
  { value: 'UTC', label: 'UTC' },
]
