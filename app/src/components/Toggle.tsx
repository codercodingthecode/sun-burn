import { Component } from 'solid-js'

interface Props {
  checked: boolean
  onChange: (val: boolean) => void
  label?: string
  id?: string
}

const Toggle: Component<Props> = (props) => {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={props.checked}
      class={`toggle ${props.checked ? 'toggle--on' : ''}`}
      onClick={() => props.onChange(!props.checked)}
      title={props.label}
    >
      <span class="toggle__thumb" />
    </button>
  )
}

export default Toggle
