// import OpenAI from 'openai'
import './shim.js'
import sol from './sol.js'

const pending = {}
let next = 0n

function poll(handle) {
  const work = pending[handle]
  if (!work) {
    return JSON.stringify({ error: `No handle ${handle}` })
  } else if (work instanceof Promise) {
    return 'delay'
  } else {
    delete pending[handle]
    return JSON.stringify(work)
  }
}

async function fetchNative(url, apiKey, body) {
  const headers = { authorization: `Bearer ${apiKey}`, 'content-type': 'application/json' }
  const reply = await fetch(url, { method: 'POST', headers, body })
  if (!reply.ok) { return { error: `HTTP ${reply.status}` } }
  return reply.json()
}

function chatCompletion(apiKey, json) {
  const handle = next++
  const work = () => {
    const url = 'https://api.openai.com/v1/chat/completions'
    return fetchNative(url, apiKey, json)
  }
  const cleanup = () => setTimeout(() => delete pending[handle], 5_000)
  pending[handle] = work().then((obj) => {
    pending[handle] = obj
  }).catch((err) => {
    pending[handle] = { error: err.message }
  }).finally(cleanup)
  return handle
}

const addressFromSeed = (seed) => sol.addressFromSeed(seed)

function getBalance(rpc, address) {
  const handle = next++
  const work = async () => {
    const lamports = await sol.getBalance(rpc, address)
    return { lamports }
  }
  const cleanup = () => setTimeout(() => delete pending[handle], 5_000)
  pending[handle] = work().then((obj) => {
    pending[handle] = obj
  }).catch((err) => {
    pending[handle] = { error: err.message }
  }).finally(cleanup)
  return handle
}

function transferFromSeed(rpc, seed, destination, amount) {
  const handle = next++
  const work = async () => {
    const signature = await sol.transferFromSeed(rpc, seed, destinateion, amount)
    return { signature }
  }
  const cleanup = () => setTimeout(() => delete pending[handle], 5_000)
  pending[handle] = work().then((obj) => {
    pending[handle] = obj
  }).catch((err) => {
    pending[handle] = { error: err.message }
  }).finally(cleanup)
  return handle
}

export const helpersInterface = {
  poll,
  chatCompletion,
  addressFromSeed,
  getBalance,
  transferFromSeed,
}
