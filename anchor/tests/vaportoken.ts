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
  createMintToInstruction,
  createTransferCheckedWithTransferHookInstruction,
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
    
  it("mints and transfers with transfer hook", async () => {
    const mint = Keypair.generate();
    const mintAuthority = Keypair.generate();
    const decimals = 9;
    const mintLen = getMintLen([ExtensionType.TransferHook]);
    const lamports = await connection.getMinimumBalanceForRentExemption(
      mintLen
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
        mintAuthority.publicKey,
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

    const initialAmount = 5n;
    const mintToIx = createMintToInstruction(
      mint.publicKey,
      payerAta,
      mintAuthority.publicKey,
      initialAmount,
      [],
      TOKEN_2022_PROGRAM_ID
    );
    await provider.sendAndConfirm(new Transaction().add(mintToIx), [
      mintAuthority,
    ]);

    const transferAmount = 2n;
    const beforeTree =
      await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);
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

    console.log("Before tree root:", beforeTree.root);
    console.log("Submitting transfer to: ", recipientAta.toBytes(), " amount: ", transferAmount.toString());

    const transferTx = new Transaction().add(transferIx);
    await provider.sendAndConfirm(transferTx, []);
    const afterTree =
      await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);

    const payerAccount = await getAccount(
      connection,
      payerAta,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    const recipientAccount = await getAccount(
      connection,
      recipientAta,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    console.log("After tree root:", afterTree.root);

    assert.notEqual(beforeTree.root, afterTree.root);
    assert.equal(
      afterTree.nextIndex.toNumber(),
      beforeTree.nextIndex.toNumber() + 1
    );
    assert.equal(payerAccount.amount, initialAmount - transferAmount);
    assert.equal(recipientAccount.amount, transferAmount);
  });

  it("deploys and calls condense", async () => {
    const mint = Keypair.generate();
    const decimals = 9;
    const mintLen = getMintLen([]);
    const lamports = await connection.getMinimumBalanceForRentExemption(
      mintLen
    );

    const [mintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("mint_authority"), mint.publicKey.toBuffer()],
      condenserProgram.programId
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
      createInitializeMintInstruction(
        mint.publicKey,
        decimals,
        mintAuthority,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(createMintTx, [mint]);

    let hasTree = true;
    try {
      await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);
    } catch {
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

    const payerAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      payer,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    const createAtaTx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        payer,
        payerAta,
        payer,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(createAtaTx, []);

    const proofBytes = Buffer.from(new Uint8Array(324)); // 256 + 4 + 64, zero commitments
    const witnessBytes = Buffer.from(new Uint8Array(12 + 34 * 32)); // header + 34 public inputs

    let threw = false;
    try {
      await condenserProgram.methods
        .condense(proofBytes, witnessBytes)
        .accountsStrict({
          mint: mint.publicKey,
          to: payerAta,
          mintAuthority,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          treeAccount,
        })
        .rpc();
    } catch (err) {
      threw = true;
      assert.match(String(err), /InvalidProof|invalid proof/);
    }

    assert.isTrue(threw, "condense should fail with InvalidProof");
  });
});
