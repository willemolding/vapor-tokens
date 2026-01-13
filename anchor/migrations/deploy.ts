// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  ExtensionType,
  TOKEN_2022_PROGRAM_ID,
  createInitializeMintInstruction,
  createInitializeTransferHookInstruction,
  getMintLen,
} from "@solana/spl-token";
import { TransferHook } from "../target/types/transfer_hook";
import { Condenser } from "../target/types/condenser";

module.exports = async function (provider: anchor.AnchorProvider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  const connection = provider.connection;
  const payer = provider.wallet.publicKey;

  const transferHookProgram = anchor.workspace
    .transferHook as Program<TransferHook>;
  const condenserProgram = anchor.workspace.condenser as Program<Condenser>;

  const mint = Keypair.generate();
  const decimals = 9;
  const mintLen = getMintLen([ExtensionType.TransferHook]);
  const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

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

  await transferHookProgram.methods.initialize().accountsStrict({
    treeAccount,
    authority: payer,
    systemProgram: SystemProgram.programId,
  }).rpc();

  console.log("Mint:", mint.publicKey.toBase58());
  console.log("Mint authority PDA:", mintAuthority.toBase58());
  console.log(
    "Transfer hook program:",
    transferHookProgram.programId.toBase58()
  );
  console.log("ExtraAccountMetaList:", extraAccountMetaList.toBase58());
  console.log("MerkleTree:", treeAccount.toBase58());
};
