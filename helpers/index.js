import OpenAI from 'openai'
import './shim.js'
import { addressFromStr, signerFromSeed, getBalance, transfer } from './sol.js'

function helperOpenAI(input) {
  return input + `456`
}

function helperSolana(input) {
  return input + `789`
}

async function fetchNative(apiKey, payload) {
  try {

    const response = await fetch('https://api.openai.com/v1/chat/completions', {
      method: 'POST',
      headers: {
        authorization: `Bearer ${apiKey}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify(payload),
    })

    const body = await response.text()
    if (response.ok) { return body }

    return JSON.stringify({ error: `OpenAI HTTP ${response.status}` })

  } catch (err) {
    return JSON.stringify({ error: err.message })
  }
}

async function chatCompletion(apiKey, json) {
  try {

    const payload = JSON.parse(json)
    const reply = await fetchNative(apiKey, payload)
    return reply

  } catch (err) {
    return JSON.stringify({ error: err.message })
  }
}

export const helpersInterface = {
  helperOpenAI,
  helperSolana,
  chatCompletion,
}
