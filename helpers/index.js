import './shim.js'
import sol from './sol.js'

const addressFromSeed = (seed) => sol.addressFromSeed(seed)

async function transferFromSeed(seed, destination, amount, lastBlock) {
  const obj = await sol.transferFromSeed(seed, destination, amount, lastBlock)
  const { signedTx, signature } = obj
  return [signedTx, signature].join(',')
}

export const helpersInterface = {
  addressFromSeed,
  transferFromSeed,
}
