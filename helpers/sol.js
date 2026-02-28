import * as ed from '@noble/ed25519'
import { sha512 } from '@noble/hashes/sha2.js'
import { address } from '@solana/kit'
import bs58 from 'bs58'

const sha512sync = (...m) => sha512(ed.etc.concatBytes(...m))

if (!ed.hashes) ed.hashes = {}
ed.hashes.sha512 = sha512sync
ed.etc.sha512Sync = sha512sync
ed.etc.sha512Async = async (...m) => sha512sync(...m)

const addressFromStr = (str) => address(str)

const textSeed32 = (seed) => {
  seed = new TextEncoder().encode(seed)
  return sha512(seed).slice(0, 32)
}

const signerFromSeed = (seed) => {
  const priv = textSeed32(seed)
  const pub = ed.getPublicKey(priv)
  const senderAddressStr = bs58.encode(pub)
  const senderAddress = address(senderAddressStr)
  const senderAddressKey = String(senderAddress)
  return Object.freeze({
    address: senderAddress,
    async signTransactions(transactions) {
      return Promise.all(
        transactions.map(async (tx) => {
          if (!tx?.messageBytes) {
            throw new Error('Transaction is missing messageBytes (cannot sign)')
          }
          const sig = await ed.sign(tx.messageBytes, priv)
          return { [senderAddressKey]: sig }
        })
      )
    }
  })
}

import {
  lamports,
  createSolanaRpc,
} from '@solana/kit'

const getBalance = async (rpc, address) => {
  rpc = createSolanaRpc(rpc)
  const { value: lamports } = await rpc.getBalance(address).send()
  return Number(lamports)
}

import {
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendTransactionWithoutConfirmingFactory,
  getSignatureFromTransaction,
} from '@solana/kit'

import { getTransferSolInstruction } from '@solana-program/system'

const transfer = async (rpc, signer, dest, amount) => {
  rpc = createSolanaRpc(rpc)
  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send()

  amount = lamports(BigInt(amount))
  const ix = getTransferSolInstruction({
    source: signer,
    destination: dest,
    amount
  })

  const txMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(signer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
    (tx) => appendTransactionMessageInstructions([ix], tx),
  )

  const signedTx = await signTransactionMessageWithSigners(txMessage)
  const sendTx = sendTransactionWithoutConfirmingFactory({ rpc })
  await sendTx(signedTx, { commitment: 'confirmed' })
  return getSignatureFromTransaction(signedTx)
}

export default { addressFromStr, signerFromSeed, getBalance, transfer }
