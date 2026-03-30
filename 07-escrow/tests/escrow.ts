import { randomBytes } from "node:crypto";
import * as anchor from "@coral-xyz/anchor";
import {
  TOKEN_2022_PROGRAM_ID,
  type TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { assert } from "chai";
import type { Escrow } from "../target/types/escrow";
import { Program } from "@coral-xyz/anchor";
import { BN } from "bn.js";

import {
  confirmTransaction,
  createAccountsMintsAndTokenAccounts,
} from "@solana-developers/helpers";

// Work on both Token Program and new Token Extensions Program
const TOKEN_PROGRAM: typeof TOKEN_2022_PROGRAM_ID | typeof TOKEN_PROGRAM_ID =
  TOKEN_2022_PROGRAM_ID;

const SECONDS = 1000;

// Tests must complete within half this time otherwise
// they are marked as slow. Since Anchor involves a little
// network IO, these tests usually take about 15 seconds.
const ANCHOR_SLOW_TEST_THRESHOLD = 40 * SECONDS;

describe("swap", () => {
  // Use the cluster and the keypair from Anchor.toml
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // See https://github.com/coral-xyz/anchor/issues/3122
  const user = (provider.wallet as anchor.Wallet).payer;
  const payer = user;

  const connection = provider.connection;

  const program = anchor.workspace.Escrow as Program<Escrow>;

  let alice: anchor.web3.Keypair;
  let bob: anchor.web3.Keypair;
  let tokenMintA: anchor.web3.Keypair;
  let tokenMintB: anchor.web3.Keypair;
  let makerTokenAccountA: anchor.web3.PublicKey;
  let makerTokenAccountB: anchor.web3.PublicKey;
  let takerTokenAccountA: anchor.web3.PublicKey;
  let takerTokenAccountB: anchor.web3.PublicKey;
  let offer: anchor.web3.PublicKey;
  let offerTokenAccount: anchor.web3.PublicKey;

  const tokenAOfferedAmount = new BN(1_000_000);
  const tokenBWantedAmount = new BN(1_000_000);

  before(
    "Creates Alice and Bob accounts, 2 token mints, and associated token accounts for both tokens for both users",
    async () => {
      const usersMintsAndTokenAccounts =
        await createAccountsMintsAndTokenAccounts(
          [
            // Alice's token balances
            [
              // 1_000_000_000 of token A
              1_000_000_000,
              // 0 of token B
              0,
            ],
            // Bob's token balances
            [
              // 0 of token A
              0,
              // 1_000_000_000 of token B
              1_000_000_000,
            ],
          ],
          1_000_000_000,
          connection,
          payer
        );

      const users = usersMintsAndTokenAccounts.users;
      alice = users[0];
      bob = users[1];

      const mints = usersMintsAndTokenAccounts.mints;
      tokenMintA = mints[0];
      tokenMintB = mints[1];

      const tokenAccounts = usersMintsAndTokenAccounts.tokenAccounts;

      const aliceTokenAccountA = tokenAccounts[0][0];
      const aliceTokenAccountB = tokenAccounts[0][1];

      const bobTokenAccountA = tokenAccounts[1][0];
      const bobTokenAccountB = tokenAccounts[1][1];

      // Save the accounts for later use
      makerTokenAccountA = aliceTokenAccountA;
      takerTokenAccountA = bobTokenAccountA;
      makerTokenAccountB = aliceTokenAccountB;
      takerTokenAccountB = bobTokenAccountB;
    }
  );

  it("Puts the tokens Alice offers into the vault when Alice makes an offer", async () => {
    // Pick a random ID for the offer we'll make
    const offerId = new BN(randomBytes(8));

    // Then determine the account addresses we'll use for the offer and the vault
    offer = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("offer"),
        alice.publicKey.toBuffer(),
        offerId.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    )[0];

    offerTokenAccount = getAssociatedTokenAddressSync(
      tokenMintA.publicKey,
      offer,
      true,
      TOKEN_PROGRAM
    );

    const transactionSignature = await program.methods
      .makeOffer(offerId, tokenAOfferedAmount, tokenBWantedAmount)
      .accounts({
        maker: alice.publicKey,
        mintA: tokenMintA.publicKey,
        mintB: tokenMintB.publicKey,
        makerTokenAccountA,
        offer,
        offerTokenAccount,
        tokenProgram: TOKEN_PROGRAM,
      })
      .signers([alice])
      .rpc();

    await confirmTransaction(connection, transactionSignature);

    // Check our vault contains the tokens offered
    const vaultBalanceResponse = await connection.getTokenAccountBalance(offerTokenAccount);
    const vaultBalance = new BN(vaultBalanceResponse.value.amount);
    assert(vaultBalance.eq(tokenAOfferedAmount));

    // Check our Offer account contains the correct data
    const offerAccount = await program.account.offer.fetch(offer);

    assert(offerAccount.maker.equals(alice.publicKey));
    assert(offerAccount.mintA.equals(tokenMintA.publicKey));
    assert(offerAccount.mintB.equals(tokenMintB.publicKey));
    assert(offerAccount.tokenBWantedAmount.eq(tokenBWantedAmount));
  }).slow(ANCHOR_SLOW_TEST_THRESHOLD);

  it("Puts the tokens from the vault into Bob's account, and gives Alice Bob's tokens, when Bob takes an offer", async () => {
    const transactionSignature = await program.methods
      .takeOffer()
      .accounts({
        taker: bob.publicKey,
        maker: alice.publicKey,
        mintA: tokenMintA.publicKey,
        mintB: tokenMintB.publicKey,
        takerTokenAccountA,
        takerTokenAccountB,
        makerTokenAccountB,
        offer,
        offerTokenAccount,
        tokenProgram: TOKEN_PROGRAM,
      })
      .signers([bob])
      .rpc();

    await confirmTransaction(connection, transactionSignature);

    // Check the offered tokens are now in Bob's account
    // (note: there is no before balance as Bob didn't have any offered tokens before the transaction)
    const bobTokenAccountBalanceAfterResponse =
      await connection.getTokenAccountBalance(takerTokenAccountA);
    const bobTokenAccountBalanceAfter = new BN(
      bobTokenAccountBalanceAfterResponse.value.amount
    );
    assert(bobTokenAccountBalanceAfter.eq(tokenAOfferedAmount));

    // Check the wanted tokens are now in Alice's account
    // (note: there is no before balance as Alice didn't have any wanted tokens before the transaction)
    const aliceTokenAccountBalanceAfterResponse =
      await connection.getTokenAccountBalance(makerTokenAccountB);
    const aliceTokenAccountBalanceAfter = new BN(
      aliceTokenAccountBalanceAfterResponse.value.amount
    );
    assert(aliceTokenAccountBalanceAfter.eq(tokenBWantedAmount));
  }).slow(ANCHOR_SLOW_TEST_THRESHOLD);
});
