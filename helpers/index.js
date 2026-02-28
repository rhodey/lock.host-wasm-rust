import OpenAI from 'openai'
import './shim.js'
import sol from './sol.js'
// import { addressFromStr, signerFromSeed, getBalance, transfer } from './sol.js'

function helperOpenAI(input) {
  return input + `456`
}

function helperSolana(input) {
  return input + `789`
}

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

async function getBalance(address) {
  try {
    address = sol.addressFromStr(address)
    const rpc = 'https://api.devnet.solana.com'
    const ok = await sol.getBalance(address, rpc)
    return String(ok)
  } catch (err) {
    return JSON.stringify({ error: err.message })
  }
}

export const helpersInterface = {
  helperOpenAI,
  helperSolana,
  chatCompletion,
  getBalance,
}
