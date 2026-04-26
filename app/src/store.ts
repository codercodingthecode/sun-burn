import { createStore } from 'solid-js/store'
import type { Manifest, Drive, FlashProgress } from './types'

export interface WizardState {
  imagePath: string | null
  manifest: Manifest | null
  // currentStep indexes into: [SelectImage, ...manifestSteps, SelectDrive, Confirm, Flashing, Done]
  currentStep: number
  fieldValues: Record<string, string>
  selectedDrive: Drive | null
  flashProgress: FlashProgress | null
  flashLog: string[]
  error: string | null
}

const initial: WizardState = {
  imagePath: null,
  manifest: null,
  currentStep: 0,
  fieldValues: {},
  selectedDrive: null,
  flashProgress: null,
  flashLog: [],
  error: null,
}

export const [state, setState] = createStore<WizardState>(initial)

export function resetWizard() {
  setState('imagePath', null)
  setState('manifest', null)
  setState('currentStep', 0)
  setState('fieldValues', {})
  setState('selectedDrive', null)
  setState('flashProgress', null)
  setState('flashLog', [])
  setState('error', null)
}

// Compute total logical steps: SelectImage + manifest steps + SelectDrive + Confirm + Flashing + Done
export function totalSteps(): number {
  return 1 + (state.manifest?.steps.length ?? 0) + 4
}

// Returns the 0-based "config step" index within manifest.steps (or null if we're not on a config step)
export function manifestStepIndex(): number | null {
  const s = state.currentStep
  if (!state.manifest) return null
  if (s >= 1 && s <= state.manifest.steps.length) return s - 1
  return null
}

// Named logical step positions
export function getStepKind(): 'select-image' | 'config' | 'select-drive' | 'confirm' | 'flashing' | 'done' {
  const s = state.currentStep
  const mLen = state.manifest?.steps.length ?? 0
  if (s === 0) return 'select-image'
  if (s >= 1 && s <= mLen) return 'config'
  if (s === mLen + 1) return 'select-drive'
  if (s === mLen + 2) return 'confirm'
  if (s === mLen + 3) return 'flashing'
  return 'done'
}

export function goNext() {
  setState('currentStep', (n) => n + 1)
}

export function goBack() {
  setState('currentStep', (n) => Math.max(0, n - 1))
}

export function setFieldValue(id: string, value: string) {
  setState('fieldValues', id, value)
}
