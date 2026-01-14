import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";
import * as fs from "fs";
import * as path from "path";
import {
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  ExtensionType,
  TOKEN_2022_PROGRAM_ID,
  AuthorityType,
  createAssociatedTokenAccountInstruction,
  createInitializeMintInstruction,
  createInitializeTransferHookInstruction,
  createMintToInstruction,
  createSetAuthorityInstruction,
  createTransferCheckedWithTransferHookInstruction,
  createTransferCheckedInstruction,
  getAccount,
  getAssociatedTokenAddressSync,
  getMintLen,
} from "@solana/spl-token";
import { Condenser } from "../target/types/condenser";
import { TransferHook } from "../target/types/transfer_hook";

describe("vapor-tokens", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const connection = provider.connection;
  const payer = provider.wallet.publicKey;

  const transferHookProgram = anchor.workspace
    .transferHook as Program<TransferHook>;
  const condenserProgram = anchor.workspace.condenser as Program<Condenser>;
    
  // it("mints and transfers with transfer hook", async () => {
  //   const mint = Keypair.generate();
  //   const mintAuthority = Keypair.generate();
  //   const decimals = 9;
  //   const mintLen = getMintLen([ExtensionType.TransferHook]);
  //   const lamports = await connection.getMinimumBalanceForRentExemption(
  //     mintLen
  //   );

  //   const [extraAccountMetaList] = PublicKey.findProgramAddressSync(
  //     [Buffer.from("extra-account-metas"), mint.publicKey.toBuffer()],
  //     transferHookProgram.programId
  //   );
  //   const [treeAccount] = PublicKey.findProgramAddressSync(
  //     [Buffer.from("merkle_tree")],
  //     transferHookProgram.programId
  //   );

  //   const createMintTx = new Transaction().add(
  //     SystemProgram.createAccount({
  //       fromPubkey: payer,
  //       newAccountPubkey: mint.publicKey,
  //       space: mintLen,
  //       lamports,
  //       programId: TOKEN_2022_PROGRAM_ID,
  //     }),
  //     createInitializeTransferHookInstruction(
  //       mint.publicKey,
  //       payer,
  //       transferHookProgram.programId,
  //       TOKEN_2022_PROGRAM_ID
  //     ),
  //     createInitializeMintInstruction(
  //       mint.publicKey,
  //       decimals,
  //       mintAuthority.publicKey,
  //       null,
  //       TOKEN_2022_PROGRAM_ID
  //     )
  //   );

  //   await provider.sendAndConfirm(createMintTx, [mint]);

  //   await transferHookProgram.methods.initialize().accountsStrict({
  //     treeAccount,
  //     authority: payer,
  //     systemProgram: SystemProgram.programId,
  //   }).rpc();

  //   await transferHookProgram.methods
  //     .initializeExtraAccountMetaList()
  //     .accountsStrict({
  //       payer,
  //       extraAccountMetaList,
  //       mint: mint.publicKey,
  //       tokenProgram: TOKEN_2022_PROGRAM_ID,
  //       associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //       systemProgram: SystemProgram.programId,
  //     })
  //     .rpc();

  //   const payerAta = getAssociatedTokenAddressSync(
  //     mint.publicKey,
  //     payer,
  //     false,
  //     TOKEN_2022_PROGRAM_ID,
  //     ASSOCIATED_TOKEN_PROGRAM_ID
  //   );
  //   const recipient = Keypair.generate();
  //   const recipientAta = getAssociatedTokenAddressSync(
  //     mint.publicKey,
  //     recipient.publicKey,
  //     false,
  //     TOKEN_2022_PROGRAM_ID,
  //     ASSOCIATED_TOKEN_PROGRAM_ID
  //   );

  //   const createAtasTx = new Transaction().add(
  //     createAssociatedTokenAccountInstruction(
  //       payer,
  //       payerAta,
  //       payer,
  //       mint.publicKey,
  //       TOKEN_2022_PROGRAM_ID,
  //       ASSOCIATED_TOKEN_PROGRAM_ID
  //     ),
  //     createAssociatedTokenAccountInstruction(
  //       payer,
  //       recipientAta,
  //       recipient.publicKey,
  //       mint.publicKey,
  //       TOKEN_2022_PROGRAM_ID,
  //       ASSOCIATED_TOKEN_PROGRAM_ID
  //     )
  //   );

  //   await provider.sendAndConfirm(createAtasTx, []);

  //   const initialAmount = 5n;
  //   const mintToIx = createMintToInstruction(
  //     mint.publicKey,
  //     payerAta,
  //     mintAuthority.publicKey,
  //     initialAmount,
  //     [],
  //     TOKEN_2022_PROGRAM_ID
  //   );
  //   await provider.sendAndConfirm(new Transaction().add(mintToIx), [
  //     mintAuthority,
  //   ]);

  //   const transferAmount = 2n;
  //   const beforeTree =
  //     await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);
  //   const transferIx = await createTransferCheckedWithTransferHookInstruction(
  //     connection,
  //     payerAta,
  //     mint.publicKey,
  //     recipientAta,
  //     payer,
  //     transferAmount,
  //     decimals,
  //     [],
  //     undefined,
  //     TOKEN_2022_PROGRAM_ID
  //   );

  //   console.log("Before tree root:", beforeTree.root);
  //   console.log("Submitting transfer to: ", recipientAta.toBytes(), " amount: ", transferAmount.toString());

  //   const transferTx = new Transaction().add(transferIx);
  //   await provider.sendAndConfirm(transferTx, []);
  //   const afterTree =
  //     await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);

  //   const payerAccount = await getAccount(
  //     connection,
  //     payerAta,
  //     undefined,
  //     TOKEN_2022_PROGRAM_ID
  //   );
  //   const recipientAccount = await getAccount(
  //     connection,
  //     recipientAta,
  //     undefined,
  //     TOKEN_2022_PROGRAM_ID
  //   );

  //   console.log("After tree root:", afterTree.root);

  //   assert.notEqual(beforeTree.root, afterTree.root);
  //   assert.equal(
  //     afterTree.nextIndex.toNumber(),
  //     beforeTree.nextIndex.toNumber() + 1
  //   );
  //   assert.equal(payerAccount.amount, initialAmount - transferAmount);
  //   assert.equal(recipientAccount.amount, transferAmount);
  // });

  it("deploys and calls condense", async () => {
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
        payer,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(createMintTx, [mint]);

    let hasTree = true;
    try {
      await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);
    } catch {
      console.log("Merkle tree account does not exist yet. Calling initialize");
      hasTree = false;
    }

    if (!hasTree) {
      await transferHookProgram.methods
        .initialize()
        .accountsStrict({
          treeAccount,
          authority: payer,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

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
    const recipientOwner = new PublicKey(
      "2ZhCrfYxvRoqai1AnbEhc31xUvVEMQGeC4ksRBn2cCtJ"
    );
    const recipientAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      recipientOwner,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    console.log("Hard-coded recipient owner:", recipientOwner.toBase58());
    console.log("Derived recipient ATA:", recipientAta.toBase58());
    const createAtaTx = new Transaction().add(
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
        recipientOwner,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(createAtaTx, []);

    const initialAmount = 10000;
    const mintToIx = createMintToInstruction(
      mint.publicKey,
      payerAta,
      payer,
      initialAmount,
      [],
      TOKEN_2022_PROGRAM_ID
    );
    await provider.sendAndConfirm(new Transaction().add(mintToIx), []);

    const setAuthorityIx = createSetAuthorityInstruction(
      mint.publicKey,
      payer,
      AuthorityType.MintTokens,
      mintAuthority,
      [],
      TOKEN_2022_PROGRAM_ID
    );
    await provider.sendAndConfirm(new Transaction().add(setAuthorityIx), []);

    const transferAmount = 1000n;
    const transferIx = await createTransferCheckedWithTransferHookInstruction(
      connection,
      payerAta,
      mint.publicKey,
      recipientAta,
      payer,
      transferAmount,
      decimals,
      [],
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    await provider.sendAndConfirm(new Transaction().add(transferIx), []);

    const proofPath = path.resolve(
      __dirname,
      "..",
      "..",
      "circuits",
      "condenser",
      "target",
      "condenser.proof"
    );
    const proofBytes = fs.readFileSync(proofPath);
    const witnessPath = path.resolve(
      __dirname,
      "..",
      "..",
      "circuits",
      "condenser",
      "target",
      "condenser.pw"
    );
    const witnessBytes = fs.readFileSync(witnessPath);

    await condenserProgram.methods
      .condense(proofBytes, witnessBytes)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
      ])
      .accountsStrict({
        mint: mint.publicKey,
        to: recipientAta,
        mintAuthority,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        treeAccount,
      })
      .rpc();

  });
});
