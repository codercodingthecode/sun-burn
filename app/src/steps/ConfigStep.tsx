import { Component, For, Show, createSignal, createEffect } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { state, setState, setFieldValue, goNext, goBack, manifestStepIndex } from '../store'
import type { Field } from '../types'
import WifiPicker from '../components/WifiPicker'
import Toggle from '../components/Toggle'
import { COUNTRIES, TIMEZONES } from '../mock'

// SSH public key files via Tauri FS (best-effort)
async function readSshKeys(): Promise<{ name: string; content: string }[]> {
  try {
    const result = await invoke<{ name: string; content: string }[]>('list_ssh_keys')
    return result
  } catch {
    return []
  }
}

const SshKeyPicker: Component<{ value: string; onChange: (v: string) => void }> = (props) => {
  const [keys, setKeys] = createSignal<{ name: string; content: string }[]>([])
  const [mode, setMode] = createSignal<'pick' | 'paste'>('pick')

  createEffect(async () => {
    const found = await readSshKeys()
    setKeys(found)
    if (found.length === 0) setMode('paste')
  })

  return (
    <div class="ssh-picker">
      <Show when={keys().length > 0}>
        <div class="ssh-tabs">
          <button class={`ssh-tab ${mode() === 'pick' ? 'ssh-tab--active' : ''}`} type="button" onClick={() => setMode('pick')}>
            ~/.ssh keys
          </button>
          <button class={`ssh-tab ${mode() === 'paste' ? 'ssh-tab--active' : ''}`} type="button" onClick={() => setMode('paste')}>
            Paste key
          </button>
        </div>
      </Show>

      <Show when={mode() === 'pick' && keys().length > 0}>
        <div class="ssh-key-list">
          <For each={keys()}>
            {(k) => (
              <button
                type="button"
                class={`ssh-key-option ${props.value === k.content ? 'ssh-key-option--selected' : ''}`}
                onClick={() => props.onChange(k.content)}
              >
                <span class="ssh-key-name">{k.name}</span>
                <span class="ssh-key-preview">{k.content.slice(0, 40)}…</span>
              </button>
            )}
          </For>
        </div>
      </Show>

      <Show when={mode() === 'paste' || keys().length === 0}>
        <textarea
          class="field-input ssh-textarea"
          placeholder="ssh-rsa AAAA... user@host"
          value={props.value}
          onInput={(e) => props.onChange(e.currentTarget.value)}
          rows={4}
          spellcheck={false}
        />
      </Show>
    </div>
  )
}

function SearchableSelect(props: {
  value: string
  onChange: (v: string) => void
  options: { value: string; label: string }[]
  placeholder?: string
}) {
  const [search, setSearch] = createSignal('')
  const [open, setOpen] = createSignal(false)

  const filtered = () => {
    const s = search().toLowerCase()
    if (!s) return props.options
    return props.options.filter((o) => o.label.toLowerCase().includes(s) || o.value.toLowerCase().includes(s))
  }

  const selected = () => props.options.find((o) => o.value === props.value)

  return (
    <div class="searchable-select">
      <button type="button" class="searchable-trigger" onClick={() => setOpen((o) => !o)}>
        <span>{selected()?.label ?? props.placeholder ?? 'Select…'}</span>
        <span class="chevron">{open() ? '▲' : '▼'}</span>
      </button>
      <Show when={open()}>
        <div class="searchable-dropdown">
          <div class="searchable-search-wrap">
            <input
              type="text"
              class="searchable-search"
              placeholder="Search…"
              value={search()}
              onInput={(e) => setSearch(e.currentTarget.value)}
              autofocus
            />
          </div>
          <div class="searchable-list">
            <For each={filtered()} fallback={<div class="searchable-empty">No results</div>}>
              {(opt) => (
                <button
                  type="button"
                  class={`searchable-option ${opt.value === props.value ? 'searchable-option--selected' : ''}`}
                  onClick={() => { props.onChange(opt.value); setOpen(false); setSearch('') }}
                >
                  {opt.label}
                </button>
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  )
}

function FieldRenderer(props: { field: Field }) {
  const f = props.field
  const value = () => state.fieldValues[f.id] ?? f.default ?? ''
  const set = (v: string) => setFieldValue(f.id, v)

  // Conditional visibility
  const visible = () => {
    if (!f.show_when) return true
    return state.fieldValues[f.show_when.field] === f.show_when.value
  }

  return (
    <Show when={visible()}>
      <div class="field-group">
        <label class="field-label" for={f.id}>
          {f.label}
          {f.required && <span class="field-required">*</span>}
        </label>

        {f.type === 'text' && (
          <input
            id={f.id}
            type="text"
            class="field-input"
            value={value()}
            onInput={(e) => set(e.currentTarget.value)}
            placeholder={f.default ?? ''}
          />
        )}

        {f.type === 'password' && (
          <input
            id={f.id}
            type="password"
            class="field-input"
            value={value()}
            onInput={(e) => set(e.currentTarget.value)}
          />
        )}

        {f.type === 'wifi-picker' && (
          <WifiPicker value={value()} onChange={set} />
        )}

        {f.type === 'ssh-key-picker' && (
          <SshKeyPicker value={value()} onChange={set} />
        )}

        {f.type === 'country-picker' && (
          <SearchableSelect
            value={value()}
            onChange={set}
            options={COUNTRIES}
            placeholder="Select country…"
          />
        )}

        {f.type === 'timezone-picker' && (
          <SearchableSelect
            value={value()}
            onChange={set}
            options={TIMEZONES}
            placeholder="Select timezone…"
          />
        )}

        {f.type === 'toggle' && (
          <div class="toggle-row">
            <Toggle
              checked={value() === 'true'}
              onChange={(v) => set(v ? 'true' : 'false')}
              id={f.id}
            />
            <span class="toggle-label-text">{value() === 'true' ? 'Enabled' : 'Disabled'}</span>
          </div>
        )}

        {f.type === 'select' && f.options && (
          <SearchableSelect
            value={value()}
            onChange={set}
            options={f.options}
            placeholder="Select…"
          />
        )}
      </div>
    </Show>
  )
}

const ConfigStep: Component = () => {
  const mIdx = () => manifestStepIndex()
  const step = () => {
    const idx = mIdx()
    if (idx === null) return null
    return state.manifest?.steps[idx] ?? null
  }

  function validate(): boolean {
    const s = step()
    if (!s) return true
    for (const field of s.fields) {
      // Check required visible fields
      const visible = !field.show_when || state.fieldValues[field.show_when.field] === field.show_when.value
      if (field.required && visible) {
        const val = state.fieldValues[field.id] ?? field.default ?? ''
        if (!val.trim()) return false
      }
    }
    return true
  }

  const [attempted, setAttempted] = createSignal(false)

  function handleNext() {
    setAttempted(true)
    if (validate()) {
      goNext()
      setAttempted(false)
    }
  }

  const isFirst = () => mIdx() === 0
  const isLast = () => {
    const idx = mIdx()
    const total = state.manifest?.steps.length ?? 0
    return idx === total - 1
  }

  return (
    <Show when={step()} fallback={<div class="step-content"><p>Loading…</p></div>}>
      {(s) => (
        <div class="step-content">
          <div class="step-header">
            <h1 class="step-title">{s().title}</h1>
            {state.manifest?.description && (
              <p class="step-subtitle">{state.manifest.description}</p>
            )}
          </div>

          <div class="fields-list">
            <For each={s().fields}>
              {(field) => <FieldRenderer field={field} />}
            </For>
          </div>

          {attempted() && !validate() && (
            <div class="field-error">Please fill in all required fields.</div>
          )}

          <div class="step-actions">
            <button type="button" class="btn btn--ghost" onClick={goBack}>
              {isFirst() ? '← Back' : '← Previous'}
            </button>
            <button type="button" class="btn btn--primary" onClick={handleNext}>
              {isLast() ? 'Continue →' : 'Next →'}
            </button>
          </div>
        </div>
      )}
    </Show>
  )
}

export default ConfigStep
