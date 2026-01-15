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

  it("deploys, transfers, and calls condense", async () => {
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
    console.log("Mint address:", mint.publicKey.toBase58());

    await transferHookProgram.methods
      .initialize()
      .accountsStrict({
        treeAccount,
        mint: mint.publicKey,
        authority: payer,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

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
      "EvFUfisEScFuZSqDXagC17m3bpP32B74dseMHtzQ5TNb"
    );
    const recipientAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      recipientOwner,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    const vaporOwner = new PublicKey(
      "DKp1YW5zcJBR4ujZnbW6gJWFXSWerS6CMJogV4tcfgNh"
    );
    const vaporAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      vaporOwner,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    const [withdrawnAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("withdrawn"), mint.publicKey.toBuffer(), recipientOwner.toBuffer()],
      condenserProgram.programId
    );

    const createRecipientAtaTx = new Transaction().add(
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
      ),
      createAssociatedTokenAccountInstruction(
        payer,
        vaporAta,
        vaporOwner,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(createRecipientAtaTx, []);

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
      vaporAta,
      payer,
      transferAmount,
      decimals,
      [],
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    await provider.sendAndConfirm(new Transaction().add(transferIx), []);

    const treeState =
      await transferHookProgram.account.merkleTreeAccount.fetch(treeAccount);
    const rootBytes = Buffer.from(treeState.root as number[]);
    console.log("Merkle root:", rootBytes.toString("hex"));

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
    const amount = witnessBytes.readBigUInt64BE(12 + 2 * 32 + 24);

    await condenserProgram.methods
      .condense(recipientOwner, proofBytes, witnessBytes)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
      ])
      .accountsStrict({
        mint: mint.publicKey,
        to: recipientAta,
        mintAuthority,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        treeAccount,
        withdrawn: withdrawnAccount,
        payer,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const recipientAccount = await getAccount(
      connection,
      recipientAta,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    assert.equal(
      recipientAccount.amount,
      transferAmount,
      "recipient balance should reflect the condensed amount"
    );

    const withdrawnState = await condenserProgram.account.withdrawnTracker.fetch(
      withdrawnAccount
    );
    assert.equal(
      withdrawnState.totalWithdrawn.toString(),
      amount.toString(),
      "withdrawn account should track the cumulative condensed amount"
    );

  });
});
