import './shim.js'
import { addressFromStr, signerFromSeed, getBalance, transfer } from './sol.js'

function helperOpenAI(input) {
  return input + `456`
}

function helperSolana(input) {
  return input + `789`
}

async function chatCompletion(apiKey, json) {
  const payload = JSON.parse(json)
  const response = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    headers: {
      authorization: `Bearer ${apiKey}`,
      'content-type': 'application/json',
    },
    body: JSON.stringify(payload),
  })

  const text = await response.text()
  if (!response.ok) {
    return JSON.stringify({
      error: {
        status: response.status,
        body: text,
      },
    })
  }

  return text
}

export const helpersInterface = {
  helperOpenAI,
  helperSolana,
  chatCompletion,
}
