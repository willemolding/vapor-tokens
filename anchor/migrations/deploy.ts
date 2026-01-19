// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  AuthorityType,
  ExtensionType,
  TOKEN_2022_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  createInitializeMetadataPointerInstruction,
  createInitializeMintInstruction,
  createInitializeTransferHookInstruction,
  createMintToInstruction,
  createSetAuthorityInstruction,
  getAssociatedTokenAddressSync,
  getMint,
  getMintLen,
  getTransferHook,
  TYPE_SIZE, LENGTH_SIZE,
} from "@solana/spl-token";
import { pack, createInitializeInstruction as createInitializeMetadataInstruction, TokenMetadata } from "@solana/spl-token-metadata";
import { TransferHook } from "../target/types/transfer_hook";
import { Condenser } from "../target/types/condenser";

module.exports = async function (provider: anchor.AnchorProvider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  const connection = provider.connection;
  const payer = provider.wallet.publicKey;
  const tokenName = process.env.TOKEN_NAME;
  const tokenSymbol = process.env.TOKEN_SYMBOL;
  const tokenUri = process.env.TOKEN_URI;
  const tokenSupplyEnv = process.env.TOKEN_SUPPLY;
  const tokenDecimalsEnv = process.env.TOKEN_DECIMALS;
  const recipientEnv = process.env.MINT_RECIPIENT;
  if (
    !tokenName ||
    !tokenSymbol ||
    !tokenUri ||
    !tokenSupplyEnv ||
    !tokenDecimalsEnv
  ) {
    throw new Error(
      "Missing TOKEN_NAME, TOKEN_SYMBOL, TOKEN_URI, TOKEN_SUPPLY, or TOKEN_DECIMALS env var"
    );
  }
  if (!recipientEnv) {
    throw new Error("Missing MINT_RECIPIENT env var");
  }
  const recipientOwner = new PublicKey(recipientEnv);
  if (!PublicKey.isOnCurve(recipientOwner)) {
    throw new Error("MINT_RECIPIENT must be an on-curve address");
  }

  const transferHookProgram = anchor.workspace
    .transferHook as Program<TransferHook>;
  const condenserProgram = anchor.workspace.condenser as Program<Condenser>;

  const mint = Keypair.generate();
  const decimals = Number(tokenDecimalsEnv);
  if (!Number.isInteger(decimals) || decimals < 0) {
    throw new Error("TOKEN_DECIMALS must be a non-negative integer");
  }
  let supply: bigint;
  try {
    supply = BigInt(tokenSupplyEnv);
  } catch {
    throw new Error("TOKEN_SUPPLY must be an integer string");
  }
  supply = supply * BigInt(10 ** decimals);
  const tokenMetadata: TokenMetadata = {
    updateAuthority: payer,
    mint: mint.publicKey,
    name: tokenName,
    symbol: tokenSymbol,
    uri: tokenUri,
    additionalMetadata: [],
  };

  console.log("Creating mint with metadata:", tokenMetadata);

  let mintLen = getMintLen(
    [
      ExtensionType.MetadataPointer,
      ExtensionType.TransferHook,
    ]
  );

  const metadataLen = TYPE_SIZE + LENGTH_SIZE + pack(tokenMetadata).length;
  const lamports = await connection.getMinimumBalanceForRentExemption(mintLen + metadataLen);

  const [mintAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("mint_authority"), mint.publicKey.toBuffer()],
    condenserProgram.programId
  );
  const [extraAccountMetaList] = PublicKey.findProgramAddressSync(
    [Buffer.from("extra-account-metas"), mint.publicKey.toBuffer()],
    transferHookProgram.programId
  );
  const [treeAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from("merkle_tree"), mint.publicKey.toBuffer()],
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
    createInitializeMetadataPointerInstruction(
      mint.publicKey,
      payer,
      mint.publicKey,
      TOKEN_2022_PROGRAM_ID
    ),
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
    ),
    createInitializeMetadataInstruction({
      programId: TOKEN_2022_PROGRAM_ID,
      metadata: mint.publicKey,
      updateAuthority: payer,
      mint: mint.publicKey,
      mintAuthority: payer,
      name: tokenMetadata.name,
      symbol: tokenMetadata.symbol,
      uri: tokenMetadata.uri,
    })
  );

  await provider.sendAndConfirm(createMintTx, [mint]);

  const mintInfo = await getMint(
    connection,
    mint.publicKey,
    undefined,
    TOKEN_2022_PROGRAM_ID
  );
  const transferHook = getTransferHook(mintInfo);
  
  console.log(
    "Transfer hook config:",
    transferHook ? transferHook.programId.toBase58() : "none"
  );

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
  const recipientAta = getAssociatedTokenAddressSync(
    mint.publicKey,
    recipientOwner,
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

  const initialAmount = supply;
  const mintToIx = createMintToInstruction(
    mint.publicKey,
    recipientAta,
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

  console.log("Mint (token address):", mint.publicKey.toBase58());
  console.log("Mint authority PDA:", mintAuthority.toBase58());
  console.log("Recipient owner:", recipientOwner.toBase58());
  console.log("Recipient ATA:", recipientAta.toBase58());
  console.log(
    "Transfer hook program:",
    transferHookProgram.programId.toBase58()
  );
  console.log("ExtraAccountMetaList:", extraAccountMetaList.toBase58());
  console.log("MerkleTree:", treeAccount.toBase58());
};
