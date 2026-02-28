import './shim.js'
import sol from './sol.js'

const encoder = new TextEncoder()

function encodeJson(value) {
  return encoder.encode(JSON.stringify(value))
}

function createJsonStream() {
  let controller
  let closed = false

  const stream = new ReadableStream({
    start(ctrl) {
      controller = ctrl
    },
  })

  const write = (value) => {
    if (closed) return
    controller.enqueue(encodeJson(value))
    controller.close()
    closed = true
  }

  const writeError = (message) => {
    const text = message instanceof Error ? message.message : String(message)
    write({ error: text })
  }

  return { stream, write, writeError }
}

function fetchNative(url, apiKey, body) {
  const headers = {
    authorization: `Bearer ${apiKey}`,
    'content-type': 'application/json',
  }

  return fetch(url, { method: 'POST', headers, body })
    .then(async (reply) => {
      const text = await reply.text()
      if (!reply.ok) {
        return { error: `OpenAI HTTP ${reply.status}: ${text}` }
      }

      try {
        const parsed = JSON.parse(text)
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
          return parsed
        }
        return { error: 'OpenAI response was not a JSON object' }
      } catch (err) {
        return { error: `OpenAI response parse failed: ${err.message}` }
      }
    })
    .catch((err) => ({ error: err.message }))
}

function chatCompletion(apiKey, json) {
  const { stream, write, writeError } = createJsonStream()

  fetchNative('https://api.openai.com/v1/chat/completions', apiKey, json)
    .then(write)
    .catch(writeError)

  return stream
}

function getBalance(rpc, address) {
  const { stream, write, writeError } = createJsonStream()

  Promise.resolve()
    .then(() => sol.addressFromStr(address))
    .then((addr) => sol.getBalance(rpc, addr))
    .then((balance) => write({ balance: Number(balance) }))
    .catch(writeError)

  return stream
}

export const helpersInterface = {
  chatCompletion,
  getBalance,
}
