import OpenAI from 'openai'
import './shim.js'
import { addressFromStr, signerFromSeed, getBalance, transfer } from './sol.js'

function helperOpenAI(input) {
  return input + `456`
}

function helperSolana(input) {
  return input + `789`
}

async function fetchNative(url, method, apiKey, body='') {
  const reply = await fetch(url, {
    method,
    headers: {
      authorization: `Bearer ${apiKey}`,
      'content-type': 'application/json',
    },
    body,
  })
  body = await reply.text()
  if (reply.ok) { return body }
  return JSON.stringify({ error: `OpenAI HTTP ${reply.status}` })
}

async function chatCompletion(apiKey, json) {
  try {
    const url = 'https://api.openai.com/v1/chat/completions'
    return await fetchNative(url, 'POST', apiKey, json)
  } catch (err) {
    return JSON.stringify({ error: err.message })
  }
}

export const helpersInterface = {
  helperOpenAI,
  helperSolana,
  chatCompletion,
}
