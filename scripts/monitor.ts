// Cron mode: run once, check switch state, fire a Discord embed if needed.
// For real-time alerts use webhook-server.ts instead.

import "dotenv/config";
import { Connection, clusterApiUrl } from "@solana/web3.js";
import { checkSwitchState } from "./shared";

const RPC_URL         = process.env.RPC_URL         ?? clusterApiUrl("devnet");
const OWNER_PUBKEY    = process.env.OWNER_PUBKEY;
const DISCORD_WEBHOOK = process.env.DISCORD_WEBHOOK;

if (!OWNER_PUBKEY)    { console.error("Missing OWNER_PUBKEY in .env");    process.exit(1); }
if (!DISCORD_WEBHOOK) { console.error("Missing DISCORD_WEBHOOK in .env"); process.exit(1); }

const connection = new Connection(RPC_URL, "confirmed");

checkSwitchState(connection, OWNER_PUBKEY, DISCORD_WEBHOOK)
  .catch(err => { console.error("Monitor failed:", err); process.exit(1); });
