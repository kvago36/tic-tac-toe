import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import {
  LAMPORTS_PER_SOL,
  SYSVAR_CLOCK_PUBKEY,
  Transaction,
  sendAndConfirmTransaction,
  SystemProgram,
} from "@solana/web3.js";
import { Test } from "../target/types/test";
import { expect, assert } from "chai";

describe("test", () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  const wallet = provider.wallet as anchor.Wallet;
  anchor.setProvider(provider);
  const another = anchor.web3.Keypair.generate();
  const game = anchor.web3.Keypair.generate();

  const program = anchor.workspace.Test as Program<Test>;

  const transferAmount = 5 * LAMPORTS_PER_SOL;

  const [PDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), wallet.publicKey.toBuffer()],
    program.programId
  );

  it("Is initialized!", async () => {
    await airdrop(connection, wallet.publicKey, 1000);

    const tx = await program.methods
      .initGame(new BN(transferAmount))
      .accounts({ user: wallet.publicKey, game: game.publicKey, vault: PDA })
      .signers([game])
      .rpc();

    const pdaAccount = await program.account.vault.fetch(PDA);
    const balance = await connection.getBalance(PDA);

    expect(wallet.publicKey.toBase58()).equal(pdaAccount.owner.toBase58());
    expect(balance).above(transferAmount);

    console.log("Init tx", tx);
  });

  it("Fetch Account", async () => {
    const pdaAccount = await program.account.vault.fetch(PDA);
    console.log(JSON.stringify(pdaAccount, null, 2));
  });

  it("Should fail because game is waiting for start!", async () => {
    try {
      await program.methods
        .claimBack()
        .accounts({
          vault: PDA,
          game: game.publicKey,
          user: wallet.publicKey,
        })
        .rpc();
      assert.fail();
    } catch (error) {
      if (error instanceof anchor.AnchorError) {
        expect(error.error.errorCode.number).equal(6005);
        expect(error.error.errorCode.code).equal("GameNotReadyToClose");
      }
    }
  });

  // TODO: shoud return SOL only if game is finished or closed
  it.skip("Should transfer SOL from PDA to the owner wallet", async () => {
    const transactionSignature = await program.methods
      .test()
      .accounts({
        vault: PDA,
        game: game.publicKey,
        user: wallet.publicKey,
      })
      .signers([wallet.payer]) // TODO: Check if this is correct because it's working without it
      .rpc();

    console.log("Send SOL from PDA to User Signature:", transactionSignature);
  });

  // TODO: GET PROPER ERROR CODE
  it("Player 1 should not join the game!", async () => {
    try {
      await program.methods
        .joinGame()
        .accounts({ user: wallet.publicKey, game: game.publicKey, vault: PDA })
        .signers([wallet.payer])
        .rpc();

      assert.fail();
    } catch (error) {
      if (error instanceof anchor.AnchorError) {
        expect(error.error.errorCode.number).equal(6002);
        expect(error.error.errorCode.code).equal("AlreadyInGame");
      }
    }
  });

  it("Player 2 should not join the game because not enough funds!", async () => {
    const _tx = await provider.connection.requestAirdrop(
      another.publicKey,
      1 * LAMPORTS_PER_SOL
    );

    await provider.connection.confirmTransaction(_tx);

    try {
      await program.methods
        .joinGame()
        .accounts({ user: another.publicKey, game: game.publicKey, vault: PDA })
        .signers([another])
        .rpc();

      assert.fail();
    } catch (error) {
      if (error instanceof anchor.AnchorError) {
        expect(error.error.errorCode.number).equal(6003);
        expect(error.error.errorCode.code).equal("InsufficientFunds");
      }
    }
  });

  it("Player 2 should join the game!", async () => {
    const _tx = await provider.connection.requestAirdrop(
      another.publicKey,
      5 * LAMPORTS_PER_SOL
    );

    await provider.connection.confirmTransaction(_tx);

    const tx = await program.methods
      .joinGame()
      .accounts({ user: another.publicKey, game: game.publicKey, vault: PDA })
      .signers([another])
      .rpc();
    // console.log("Join game tx", tx);
  });
});

export async function airdrop(
  connection: any,
  address: any,
  amount = 500_000_000_000
) {
  await connection.confirmTransaction(
    await connection.requestAirdrop(address, amount),
    "confirmed"
  );
}
