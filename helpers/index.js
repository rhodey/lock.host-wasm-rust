import OpenAI from 'openai'
import './shim.js'
import { addressFromStr, signerFromSeed, getBalance, transfer } from './sol.js'

function helperOpenAI(input) {
  return input + `456`
}

function helperSolana(input) {
  return input + `789`
}

async function chatCompletion(apiKey, json) {
  json = JSON.parse(json)
  const client = new OpenAI({ apiKey, dangerouslyAllowBrowser: true })
  const reply = await client.chat.completions.create(json)
  return JSON.stringify(reply)
}

export const helpersInterface = {
  helperOpenAI,
  helperSolana,
  chatCompletion,
}
