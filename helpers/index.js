import './shim.js'
import { addressFromStr, signerFromSeed, getBalance, transfer } from './sol.js'

function helperOpenAI(input) {
  return input + `456`
}

function helperSolana(input) {
  return input + `789`
}

async function chatCompletion(apiKey, json) {
  try {
    const payload = JSON.parse(json)

    // NOTE: In this WASI component runtime, outbound fetch/OpenAI client calls
    // can trap at the host boundary (wasi:http fields conversion), which causes
    // the whole request to become HTTP 500 before JS can catch the error.
    // Return a wrapped error payload instead of trapping.
    return JSON.stringify({
      error:
        'chat completion unavailable: outbound OpenAI HTTP is not supported in this runtime without additional host capabilities',
      model: payload?.model,
      messageCount: Array.isArray(payload?.messages) ? payload.messages.length : 0,
      hasApiKey: Boolean(apiKey),
    })
  } catch (err) {
    return JSON.stringify({ error: err.message })
  }
}

export const helpersInterface = {
  helperOpenAI,
  helperSolana,
  chatCompletion,
}
