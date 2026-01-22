// Runs inside a dedicated Web Worker.
// Vite will bundle this as a module worker.

import init, { solve } from '../wasm/durak_wasm.js'

let ready = false

function postError(err, context, id) {
  const msg = err instanceof Error ? err : new Error(String(err))
  self.postMessage({
    type: 'error',
    id,
    context,
    message: msg.message,
    stack: msg.stack || '',
  })
}

self.onmessage = async (e) => {
  const msg = e.data

  try {
    if (msg?.type === 'init') {
      if (!ready) {
        await init()
        ready = true
      }
      self.postMessage({ type: 'ready' })
      return
    }

    if (msg?.type === 'solve') {
      if (!ready) {
        await init()
        ready = true
      }

      const startedAt = performance.now()
      const result = solve(msg.req)
      const elapsedMs = performance.now() - startedAt

      self.postMessage({ type: 'result', id: msg.id, result, elapsedMs })
      return
    }
  } catch (err) {
    postError(err, msg?.type || 'unknown', msg?.id)
  }
}
