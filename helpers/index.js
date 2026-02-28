import OpenAI from 'openai'
import './shim.js'
import { addressFromStr, signerFromSeed, getBalance, transfer } from './sol.js'

function helperOpenAI(input) {
  return input + `456`
}

function helperSolana(input) {
  return input + `789`
}

function sanitizeHeaders(headers) {
  if (!headers) return undefined

  const normalized = new Headers(headers)
  const out = {}
  for (const [key, value] of normalized.entries()) {
    if (value == null) continue
    out[key] = String(value)
  }
  return out
}

async function safeFetch(input, init = {}) {
  const safeInit = {
    ...init,
    headers: sanitizeHeaders(init.headers),
  }

  return fetch(input, safeInit)
}

async function chatCompletion(apiKey, json) {
  try {
    const payload = JSON.parse(json)
    const client = new OpenAI({
      apiKey,
      dangerouslyAllowBrowser: true,
      fetch: safeFetch,
    })

    const reply = await client.chat.completions.create(payload)
    return JSON.stringify(reply)
  } catch (err) {
    return JSON.stringify({ error: err?.message ?? String(err) })
  }
}

export const helpersInterface = {
  helperOpenAI,
  helperSolana,
  chatCompletion,
}
