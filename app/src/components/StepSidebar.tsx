import { Component, For, Show } from 'solid-js'
import { state, getStepKind, manifestStepIndex } from '../store'

// Static step definitions for sidebar display
function getSidebarSteps() {
  const manifest = state.manifest
  const steps: { id: string; label: string; icon: string }[] = [
    { id: 'select-image', label: 'Select Image', icon: '⊞' },
  ]
  if (manifest) {
    for (const s of manifest.steps) {
      steps.push({ id: `config-${s.id}`, label: s.title, icon: '◈' })
    }
  }
  steps.push({ id: 'select-drive', label: 'Select Drive', icon: '⊡' })
  steps.push({ id: 'confirm', label: 'Confirm', icon: '⊙' })
  steps.push({ id: 'flashing', label: 'Flashing', icon: '⊗' })
  steps.push({ id: 'done', label: 'Done', icon: '✓' })
  return steps
}

function currentSidebarIndex(): number {
  const kind = getStepKind()
  const manifest = state.manifest
  const mIdx = manifestStepIndex()

  if (kind === 'select-image') return 0
  if (kind === 'config' && mIdx !== null) return 1 + mIdx
  const mLen = manifest?.steps.length ?? 0
  if (kind === 'select-drive') return 1 + mLen
  if (kind === 'confirm') return 2 + mLen
  if (kind === 'flashing') return 3 + mLen
  return 4 + mLen
}

const StepSidebar: Component = () => {
  const steps = () => getSidebarSteps()
  const activeIdx = () => currentSidebarIndex()

  return (
    <aside class="sidebar">
      <div class="sidebar-brand">sun-burn</div>

      <Show when={state.manifest}>
        {(mf) => (
          <div class="sidebar-manifest">
            <span class="sidebar-manifest-name">{mf().name}</span>
            {mf().description && (
              <span class="sidebar-manifest-desc">{mf().description}</span>
            )}
          </div>
        )}
      </Show>

      <nav class="sidebar-steps">
        <For each={steps()}>
          {(step, idx) => {
            const status = () => {
              const a = activeIdx()
              if (idx() < a) return 'done'
              if (idx() === a) return 'active'
              return 'future'
            }
            return (
              <div class={`sidebar-step sidebar-step--${status()}`}>
                <span class="sidebar-step-icon">
                  {status() === 'done' ? (
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                      <path d="M2.5 7l3 3 6-6" stroke="#f59e0b" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                  ) : (
                    <span>{step.icon}</span>
                  )}
                </span>
                <span class="sidebar-step-label">{step.label}</span>
              </div>
            )
          }}
        </For>
      </nav>

      <div class="sidebar-footer">
        <Show when={state.error}>
          <div class="sidebar-error">{state.error}</div>
        </Show>
      </div>
    </aside>
  )
}

export default StepSidebar
