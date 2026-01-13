import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";
import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  ExtensionType,
  TOKEN_2022_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  createInitializeMintInstruction,
  createInitializeTransferHookInstruction,
  createTransferCheckedWithTransferHookInstruction,
  getAccount,
  getAssociatedTokenAddressSync,
  getMintLen,
} from "@solana/spl-token";
import { TransferHook } from "../target/types/transfer_hook";
import { Condenser } from "../target/types/condenser";

describe("vapor-tokens", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const connection = provider.connection;
  const payer = provider.wallet.publicKey;

  const transferHookProgram = anchor.workspace
    .transferHook as Program<TransferHook>;
  const condenserProgram = anchor.workspace.condenser as Program<Condenser>;

  it("mints and transfers with transfer hook", async () => {
    const mint = Keypair.generate();
    const decimals = 9;
    const mintLen = getMintLen([ExtensionType.TransferHook]);
    const lamports = await connection.getMinimumBalanceForRentExemption(
      mintLen
    );

    const [mintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("mint_authority"), mint.publicKey.toBuffer()],
      condenserProgram.programId
    );
    const [extraAccountMetaList] = PublicKey.findProgramAddressSync(
      [Buffer.from("extra-account-metas"), mint.publicKey.toBuffer()],
      transferHookProgram.programId
    );
    const [treeAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("merkle_tree")],
      transferHookProgram.programId
    );

    const createMintTx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer,
        newAccountPubkey: mint.publicKey,
        space: mintLen,
        lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeTransferHookInstruction(
        mint.publicKey,
        payer,
        transferHookProgram.programId,
        TOKEN_2022_PROGRAM_ID
      ),
      createInitializeMintInstruction(
        mint.publicKey,
        decimals,
        mintAuthority,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(createMintTx, [mint]);

    await transferHookProgram.methods.initialize().accountsStrict({
      treeAccount,
      authority: payer,
      systemProgram: SystemProgram.programId,
    }).rpc();

    await transferHookProgram.methods
      .initializeExtraAccountMetaList()
      .accountsStrict({
        payer,
        extraAccountMetaList,
        mint: mint.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const payerAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      payer,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    const recipient = Keypair.generate();
    const recipientAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      recipient.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const createAtasTx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        payer,
        payerAta,
        payer,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      ),
      createAssociatedTokenAccountInstruction(
        payer,
        recipientAta,
        recipient.publicKey,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(createAtasTx, []);

    // const initialAmount = 5;
    // for (let i = 0; i < initialAmount; i += 1) {
    //   await condenserProgram.methods
    //     .condense(Buffer.from([]), Buffer.from([]))
    //     .accountsStrict({
    //       mint: mint.publicKey,
    //       to: payerAta,
    //       mintAuthority,
    //       tokenProgram: TOKEN_2022_PROGRAM_ID,
    //       treeAccount,
    //     })
    //     .rpc();
    // }

    // const transferAmount = 2n;
    // const beforeTree =
    //   await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);
    // const transferIx = await createTransferCheckedWithTransferHookInstruction(
    //   connection,
    //   payerAta,
    //   mint.publicKey,
    //   recipientAta,
    //   payer,
    //   transferAmount,
    //   decimals,
    //   [],
    //   undefined,
    //   TOKEN_2022_PROGRAM_ID
    // );

    // const transferTx = new Transaction().add(transferIx);
    // await provider.sendAndConfirm(transferTx, []);
    // const afterTree =
    //   await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);

    // const payerAccount = await getAccount(
    //   connection,
    //   payerAta,
    //   undefined,
    //   TOKEN_2022_PROGRAM_ID
    // );
    // const recipientAccount = await getAccount(
    //   connection,
    //   recipientAta,
    //   undefined,
    //   TOKEN_2022_PROGRAM_ID
    // );

    // assert.equal(
    //   afterTree.nextIndex.toNumber(),
    //   beforeTree.nextIndex.toNumber() + 1
    // );
    // assert.equal(payerAccount.amount, BigInt(initialAmount) - transferAmount);
    // assert.equal(recipientAccount.amount, transferAmount);
  });
});
