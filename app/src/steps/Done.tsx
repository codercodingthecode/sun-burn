import { Component } from 'solid-js'
import { state, resetWizard } from '../store'

const Done: Component = () => {
  return (
    <div class="step-content step-done">
      <div class="done-icon-wrap">
        <svg width="80" height="80" viewBox="0 0 80 80" fill="none">
          <circle cx="40" cy="40" r="36" stroke="#f59e0b" stroke-width="2" opacity="0.25" />
          <circle cx="40" cy="40" r="28" stroke="#f59e0b" stroke-width="2" opacity="0.5" />
          <circle cx="40" cy="40" r="20" stroke="#f59e0b" stroke-width="2" />
          <path
            d="M28 40l8 8 16-16"
            stroke="#f59e0b"
            stroke-width="3"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </div>

      <h1 class="done-title">Image written successfully</h1>

      <p class="done-sub">
        {state.selectedDrive?.display_name ?? 'Drive'} is ready.
      </p>

      <div class="done-steps">
        <div class="done-step">
          <span class="done-step-num">1</span>
          <span>Eject the drive from this computer</span>
        </div>
        <div class="done-step">
          <span class="done-step-num">2</span>
          <span>Insert it into your device</span>
        </div>
        <div class="done-step">
          <span class="done-step-num">3</span>
          <span>Power on — it should boot within 30 seconds</span>
        </div>
      </div>

      <button
        type="button"
        class="done-restart"
        onClick={resetWizard}
      >
        Flash another image
      </button>
    </div>
  )
}

export default Done
