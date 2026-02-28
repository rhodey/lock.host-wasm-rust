import OpenAI from 'openai'
import './shim.js'
import sol from './sol.js'

async function fetchNative(url, apiKey, body) {
  const headers = { authorization: `Bearer ${apiKey}`, 'content-type': 'application/json' }
  const reply = await fetch(url, { method: 'POST', headers, body })
  body = await reply.text()
  if (reply.ok) { return body }
  return JSON.stringify({ error: `OpenAI HTTP ${reply.status}` })
}

async function chatCompletion(apiKey, json) {
  try {
    const url = 'https://api.openai.com/v1/chat/completions'
    return await fetchNative(url, apiKey, json)
  } catch (err) {
    return JSON.stringify({ error: err.message })
  }
}

async function getBalance(rpc, address) {
  console.log(123, address)
  try {
    address = sol.addressFromStr(address)
    console.log(456, String(address))
    const ok = await sol.getBalance(rpc, address)
    return String(ok)
  } catch (err) {
    return JSON.stringify({ error: err.message })
  }
}

export const helpersInterface = {
  chatCompletion,
  getBalance,
}
