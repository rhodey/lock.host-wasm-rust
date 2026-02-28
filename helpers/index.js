import OpenAI from 'openai'
import './shim.js'
import sol from './sol.js'

const encoder = new TextEncoder()

function toJsonStream(value) {
  const text = JSON.stringify(value)
  const bytes = encoder.encode(text)
  let offset = 0

  return {
    read(len) {
      const size = Number(len)
      if (offset >= bytes.length) return new Uint8Array(0)
      const end = Number.isFinite(size) && size >= 0 ? offset + size : bytes.length
      const chunk = bytes.slice(offset, Math.min(end, bytes.length))
      offset += chunk.length
      return chunk
    },
    blockingRead(len) {
      return this.read(len)
    },
  }
}

async function fetchNative(url, apiKey, body) {
  const headers = { authorization: `Bearer ${apiKey}`, 'content-type': 'application/json' }
  const reply = await fetch(url, { method: 'POST', headers, body })
  body = await reply.text()
  if (!reply.ok) {
    return { error: `OpenAI HTTP ${reply.status}: ${body}` }
  }

  try {
    const parsed = JSON.parse(body)
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed
    }
    return { error: 'OpenAI response was not a JSON object' }
  } catch (err) {
    return { error: `OpenAI response parse failed: ${err.message}` }
  }
}

async function chatCompletion(apiKey, json) {
  try {
    const url = 'https://api.openai.com/v1/chat/completions'
    const payload = await fetchNative(url, apiKey, json)
    return toJsonStream(payload)
  } catch (err) {
    return toJsonStream({ error: err.message })
  }
}

async function getBalance(rpc, address) {
  try {
    address = sol.addressFromStr(address)
    const value = await sol.getBalance(rpc, address)
    return toJsonStream({ balance: Number(value) })
  } catch (err) {
    return toJsonStream({ error: err.message })
  }
}

export const helpersInterface = {
  chatCompletion,
  getBalance,
}
