import { Component, Switch, Match } from 'solid-js'
import { getStepKind } from './store'
import StepSidebar from './components/StepSidebar'
import SelectImage from './steps/SelectImage'
import ConfigStep from './steps/ConfigStep'
import SelectDrive from './steps/SelectDrive'
import Confirm from './steps/Confirm'
import Flashing from './steps/Flashing'
import Done from './steps/Done'

const App: Component = () => {
  return (
    <div class="app-shell">
      <StepSidebar />
      <main class="app-main">
        <Switch>
          <Match when={getStepKind() === 'select-image'}>
            <SelectImage />
          </Match>
          <Match when={getStepKind() === 'config'}>
            <ConfigStep />
          </Match>
          <Match when={getStepKind() === 'select-drive'}>
            <SelectDrive />
          </Match>
          <Match when={getStepKind() === 'confirm'}>
            <Confirm />
          </Match>
          <Match when={getStepKind() === 'flashing'}>
            <Flashing />
          </Match>
          <Match when={getStepKind() === 'done'}>
            <Done />
          </Match>
        </Switch>
      </main>
    </div>
  )
}

export default App
