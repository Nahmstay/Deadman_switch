// Helius event-driven mode: Helius POSTs here whenever the switch PDA changes.
// Re-fetches state from the RPC on each call so the notification is always fresh.
// See the README for setup instructions.

import "dotenv/config";
import http from "http";
import { Connection, clusterApiUrl } from "@solana/web3.js";
import { checkSwitchState } from "./shared";

// ── Config ────────────────────────────────────────────────────────────────────

const RPC_URL          = process.env.RPC_URL          ?? clusterApiUrl("devnet");
const OWNER_PUBKEY     = process.env.OWNER_PUBKEY;
const DISCORD_WEBHOOK  = process.env.DISCORD_WEBHOOK;
const HELIUS_AUTH      = process.env.HELIUS_AUTH_HEADER; // match what you set in Helius UI
const PORT             = parseInt(process.env.WEBHOOK_PORT ?? "3000", 10);

if (!OWNER_PUBKEY)    { console.error("Missing OWNER_PUBKEY in .env");    process.exit(1); }
if (!DISCORD_WEBHOOK) { console.error("Missing DISCORD_WEBHOOK in .env"); process.exit(1); }

const connection = new Connection(RPC_URL, "confirmed");

// ── Cooldown ──────────────────────────────────────────────────────────────────
// Deduplicates rapid-fire webhooks (e.g. deposit + check_in back to back).

let lastNotifiedAt = 0;
const COOLDOWN_MS  = 60_000; // 1 minute

async function handleWebhook(txSignature?: string): Promise<void> {
  const now = Date.now();
  if (now - lastNotifiedAt < COOLDOWN_MS) {
    console.log("Cooldown active; skipping duplicate.");
    return;
  }
  lastNotifiedAt = now;

  if (txSignature) {
    console.log(`Processing state change triggered by tx: ${txSignature}`);
  }

  await checkSwitchState(connection, OWNER_PUBKEY!, DISCORD_WEBHOOK!);
}

// ── HTTP server ───────────────────────────────────────────────────────────────

const server = http.createServer((req, res) => {
  // Uptime / health check
  if (req.method === "GET" && req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "ok", owner: OWNER_PUBKEY, port: PORT }));
    return;
  }

  if (req.method !== "POST" || req.url !== "/webhook") {
    res.writeHead(404);
    res.end("Not found");
    return;
  }

  // Check auth header if HELIUS_AUTH is set.
  if (HELIUS_AUTH && req.headers["authorization"] !== HELIUS_AUTH) {
    console.warn(`[${new Date().toISOString()}] Rejected webhook: invalid authorization header`);
    res.writeHead(401);
    res.end("Unauthorized");
    return;
  }

  // Drain the body; must happen to close the connection.
  const chunks: Buffer[] = [];
  req.on("data", (chunk: Buffer) => chunks.push(chunk));

  req.on("end", () => {
    // Respond immediately; Helius retries if no 2xx within 5s.
    res.writeHead(200);
    res.end("OK");

    // Grab tx signature for logging if available.
    let txSignature: string | undefined;
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString());
      const first = Array.isArray(body) ? body[0] : body;
      txSignature = first?.signature;
      console.log(
        `[${new Date().toISOString()}] Helius webhook received` +
        (txSignature ? `, sig: ${txSignature.slice(0, 16)}…` : "")
      );
    } catch {
      console.log(`[${new Date().toISOString()}] Helius webhook received`);
    }

    // Fire async; re-fetches state from RPC.
    handleWebhook(txSignature).catch(err =>
      console.error("Notification error:", err)
    );
  });
});

server.listen(PORT, () => {
  console.log("");
  console.log("  Deadman Switch: Helius Webhook Server");
  console.log(`  Listening on port ${PORT}`);
  console.log(`  Watching:  ${OWNER_PUBKEY}`);
  console.log(`  Endpoints: POST /webhook  |  GET /health`);
  console.log(`  Auth:      ${HELIUS_AUTH ? "HELIUS_AUTH_HEADER is set ✓" : "NOT SET (any caller can fire notifications ⚠️)"}`);
  console.log("");
});
