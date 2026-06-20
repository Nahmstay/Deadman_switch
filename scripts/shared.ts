// Shared account parsing and notification logic used by monitor.ts and webhook-server.ts.

import {
  Connection,
  PublicKey,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";

// ── Constants ─────────────────────────────────────────────────────────────────

export const PROGRAM_ID  = new PublicKey("5BR3iY7HKjy2qdL7i99bQZPP9fxAzpGAVcihR7MHZWj6");
export const SWITCH_SEED = Buffer.from("deadman-switch");
export const REMINDER_DAYS = 10;

// ── Types ─────────────────────────────────────────────────────────────────────

export interface BeneficiaryInfo {
  address: string;
  shareBps: number;
}

export interface SwitchState {
  owner: string;
  beneficiaries: BeneficiaryInfo[];
  checkInInterval: bigint;
  gracePeriod: bigint;
  lastCheckIn: bigint;
  triggered: boolean;
  balanceLamports: number;
}

export interface EmbedField { name: string; value: string; inline?: boolean; }
export interface Embed {
  title: string;
  description: string;
  color: number;
  fields?: EmbedField[];
  timestamp?: string;
  footer?: { text: string };
}

// ── Account parsing ───────────────────────────────────────────────────────────
//
// Manual borsh layout of DeadmanSwitch (field order matters):
//   [8]         discriminator
//   [32]        owner Pubkey
//   [4+N*34]    Vec<Beneficiary>  (each: 32 pubkey + 2 u16 share_bps)
//   [8]         check_in_interval  i64
//   [8]         grace_period       i64
//   [8]         last_check_in      i64
//   [1]         triggered          bool
//   [1+(4+len)] Option<String>     message
//   [1]         bump               u8

export function parse(data: Buffer, balanceLamports: number): SwitchState {
  let offset = 8; // skip 8-byte Anchor discriminator

  const owner = new PublicKey(data.slice(offset, offset + 32)).toBase58();
  offset += 32;

  const numBeneficiaries = data.readUInt32LE(offset);
  offset += 4;

  const beneficiaries: BeneficiaryInfo[] = [];
  for (let i = 0; i < numBeneficiaries; i++) {
    const address = new PublicKey(data.slice(offset, offset + 32)).toBase58();
    offset += 32;
    const shareBps = data.readUInt16LE(offset);
    offset += 2;
    beneficiaries.push({ address, shareBps });
  }

  const checkInInterval = data.readBigInt64LE(offset); offset += 8;
  const gracePeriod     = data.readBigInt64LE(offset); offset += 8;
  const lastCheckIn     = data.readBigInt64LE(offset); offset += 8;
  const triggered       = data[offset] !== 0;

  return { owner, beneficiaries, checkInInterval, gracePeriod, lastCheckIn, triggered, balanceLamports };
}

// ── Formatting helpers ────────────────────────────────────────────────────────

export function bpsToPercent(bps: number): string {
  return (bps / 100).toFixed(2) + "%";
}

export function beneficiaryList(beneficiaries: BeneficiaryInfo[]): string {
  return beneficiaries
    .map((b, i) => `**${i + 1}.** \`${b.address}\`: ${bpsToPercent(b.shareBps)}`)
    .join("\n");
}

// ── Discord ───────────────────────────────────────────────────────────────────

export async function sendWebhook(webhookUrl: string, embed: Embed): Promise<void> {
  const res = await fetch(webhookUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ embeds: [embed] }),
  });
  if (!res.ok) {
    console.error("Discord webhook failed:", res.status, await res.text());
  } else {
    console.log("Discord notification sent.");
  }
}

// ── Core state check ──────────────────────────────────────────────────────────

export async function checkSwitchState(
  connection: Connection,
  ownerPubkeyStr: string,
  discordWebhookUrl: string
): Promise<void> {
  const owner = new PublicKey(ownerPubkeyStr);

  const [switchPda] = await PublicKey.findProgramAddress(
    [SWITCH_SEED, owner.toBuffer()],
    PROGRAM_ID
  );

  console.log("Checking switch:", switchPda.toBase58());

  const accountInfo = await connection.getAccountInfo(switchPda);
  if (!accountInfo) {
    console.log("Switch account not found; not yet initialized.");
    return;
  }

  const s             = parse(Buffer.from(accountInfo.data), accountInfo.lamports);
  const balanceSol    = (s.balanceLamports / LAMPORTS_PER_SOL).toFixed(4);
  const nowSecs       = BigInt(Math.floor(Date.now() / 1000));
  const deadlineSecs  = s.lastCheckIn + s.checkInInterval;
  const graceEndSecs  = deadlineSecs + s.gracePeriod;
  const secsToDeadline  = deadlineSecs - nowSecs;
  const secsToGraceEnd  = graceEndSecs - nowSecs;
  const daysToDeadline  = Number(secsToDeadline) / 86_400;
  const daysToGraceEnd  = Number(secsToGraceEnd) / 86_400;
  const deadlineDate  = new Date(Number(deadlineSecs) * 1000).toUTCString();
  const graceEndDate  = new Date(Number(graceEndSecs) * 1000).toUTCString();
  const graceDays     = Number(s.gracePeriod) / 86_400;
  const n             = s.beneficiaries.length;

  // ── Case 1: already triggered ──────────────────────────────────────────────
  if (s.triggered) {
    console.log("Switch has already been triggered.");
    await sendWebhook(discordWebhookUrl, {
      title: "🔴 Deadman Switch: Triggered",
      description:
        `The deadman switch has fired. All available funds have been distributed to the beneficiar${n !== 1 ? "ies" : "y"} below.\n\n` +
        `The on-chain account remains as a permanent record.`,
      color: 0xff0000,
      fields: [
        { name: `Beneficiar${n !== 1 ? "ies" : "y"} & Splits`, value: beneficiaryList(s.beneficiaries), inline: false },
        { name: "Switch PDA",               value: switchPda.toBase58(), inline: false },
        { name: "Residual balance (rent)",  value: `${balanceSol} SOL`,  inline: true },
      ],
      timestamp: new Date().toISOString(),
    });
    return;
  }

  // ── Case 2: fully expired — trigger window open ────────────────────────────
  if (secsToGraceEnd <= 0n) {
    console.warn("Grace period expired; anyone can call trigger().");
    await sendWebhook(discordWebhookUrl, {
      title: "🚨 Deadman Switch: Fully Expired",
      description:
        `The check-in deadline **and** the ${graceDays.toFixed(0)}-day grace period have both passed.\n\n` +
        `Anyone can now call \`trigger()\` to release funds to the beneficiar${n !== 1 ? "ies" : "y"} below.`,
      color: 0xff2200,
      fields: [
        { name: `Beneficiar${n !== 1 ? "ies" : "y"} & Splits`, value: beneficiaryList(s.beneficiaries), inline: false },
        { name: "Balance at risk", value: `${balanceSol} SOL`, inline: true },
        { name: "Deadline was",    value: deadlineDate,        inline: true },
        { name: "Grace ended",     value: graceEndDate,        inline: true },
        { name: "Switch PDA",      value: switchPda.toBase58(), inline: false },
      ],
      timestamp: new Date().toISOString(),
    });
    return;
  }

  // ── Case 3: in the grace window ────────────────────────────────────────────
  if (secsToDeadline <= 0n) {
    const daysLeft = Math.ceil(daysToGraceEnd);
    console.warn(`In grace period; ${daysLeft} day(s) left.`);
    await sendWebhook(discordWebhookUrl, {
      title: `🟠 Deadman Switch: Grace Period (${daysLeft} Day${daysLeft !== 1 ? "s" : ""} Left)`,
      description:
        `The check-in deadline has **passed**, but the ${graceDays.toFixed(0)}-day grace window is still open.\n\n` +
        `**Check in now to reset the full timer**, or call \`cancel()\` to recover your funds.\n\n` +
        `If neither happens within **${daysLeft} day${daysLeft !== 1 ? "s" : ""}**, anyone can trigger the switch and funds will go to:`,
      color: 0xff8800,
      fields: [
        { name: `Beneficiar${n !== 1 ? "ies" : "y"} & Splits`, value: beneficiaryList(s.beneficiaries), inline: false },
        { name: "Balance",    value: `${balanceSol} SOL`, inline: true },
        { name: "Grace ends", value: graceEndDate,        inline: true },
        { name: "Switch PDA", value: switchPda.toBase58(), inline: false },
      ],
      timestamp: new Date().toISOString(),
      footer: { text: "Check in or cancel before the grace period ends." },
    });
    return;
  }

  // ── Case 4: approaching deadline, within reminder window ───────────────────
  const reminderThresholdSecs = BigInt(REMINDER_DAYS * 86_400);
  if (secsToDeadline <= reminderThresholdSecs) {
    const daysLeft = Math.ceil(daysToDeadline);
    console.log(`Reminder: ${daysLeft} day(s) until check-in deadline.`);
    await sendWebhook(discordWebhookUrl, {
      title: `⏰ Deadman Switch: Check-in Due in ${daysLeft} Day${daysLeft !== 1 ? "s" : ""}`,
      description:
        `The switch deadline is approaching. **Check in within ${daysLeft} day${daysLeft !== 1 ? "s" : ""}** ` +
        `or the ${graceDays.toFixed(0)}-day grace period will begin.\n\n` +
        `If the grace period also lapses, funds will be released to:`,
      color: 0xffaa00,
      fields: [
        { name: `Beneficiar${n !== 1 ? "ies" : "y"} & Splits`, value: beneficiaryList(s.beneficiaries), inline: false },
        { name: "Balance at risk",        value: `${balanceSol} SOL`, inline: true },
        { name: "Check-in deadline",      value: deadlineDate,         inline: true },
        { name: "Grace ends (if missed)", value: graceEndDate,         inline: true },
        { name: "Switch PDA",             value: switchPda.toBase58(), inline: false },
      ],
      timestamp: new Date().toISOString(),
      footer: { text: "Check in to reset the timer." },
    });
    return;
  }

  // ── All good ───────────────────────────────────────────────────────────────
  console.log(
    `OK: ${daysToDeadline.toFixed(1)} days until deadline (${deadlineDate}). ` +
    `Grace ends ${daysToGraceEnd.toFixed(1)} days out. Balance: ${balanceSol} SOL.`
  );
}
