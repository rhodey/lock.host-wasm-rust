import * as ed from '@noble/ed25519'
import { sha512 } from '@noble/hashes/sha2.js'
import { address } from '@solana/kit'
import bs58 from 'bs58'

const sha512sync = (...m) => sha512(ed.etc.concatBytes(...m))

if (!ed.hashes) ed.hashes = {}
ed.hashes.sha512 = sha512sync
ed.etc.sha512Sync = sha512sync
ed.etc.sha512Async = async (...m) => sha512sync(...m)

const textSeed32 = (seed) => {
  seed = new TextEncoder().encode(seed)
  return sha512(seed).slice(0, 32)
}

const addressFromSeed = (seed) => {
  const priv = textSeed32(seed)
  const pub = ed.getPublicKey(priv)
  return bs58.encode(pub)
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
  lamports, pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  getBase64EncodedWireTransaction,
  getSignatureFromTransaction,
} from '@solana/kit'

import { getTransferSolInstruction } from '@solana-program/system'

const transferFromSeed = async (seed, destination, amount, lastBlock, lastHeight) => {
  const signer = signerFromSeed(seed)
  destination = address(destination)
  amount = lamports(BigInt(amount))

  const ix = getTransferSolInstruction({
    source: signer,
    destination,
    amount
  })

  lastBlock = {
    blockhash: lastBlock,
    lastValidBlockHeight: lastHeight
  }

  const txMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(signer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(lastBlock, tx),
    (tx) => appendTransactionMessageInstructions([ix], tx),
  )

  let signedTx = await signTransactionMessageWithSigners(txMessage)
  const signature = getSignatureFromTransaction(signedTx)
  signedTx = getBase64EncodedWireTransaction(signedTx)
  return { signedTx, signature }
}

export default { addressFromSeed, transferFromSeed }
