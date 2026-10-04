# LinkedIn Posts — `stocks` project

Three posts about the real-time stock-quote streaming client/server, each with a
different angle so they can be published as a short series.

---

## Post 1 — Build announcement (broad, approachable)

🦀 I built a real-time stock-quote streaming engine in pure Rust — no async runtime, no framework, just the standard library and raw sockets.

The idea is simple: a **server** generates live quotes and broadcasts them; a **client** subscribes to just the tickers it cares about and receives a filtered live stream.

What made it fun was doing it the hard way — hand-rolling the concurrency instead of reaching for Tokio:

• A single generator thread produces quotes
• A broadcaster fans them out to per-client channels (`mpsc::sync_channel`)
• Every connected client gets its own streaming thread
• Shared state guarded by `Arc<RwLock<…>>` (parking_lot, to dodge lock poisoning)

It's a learning project, but I treated it like production: CI/CD on GitHub Actions, automated cross-platform releases, Dependabot auto-merge, pre-commit hooks, and an MSRV check.

Sometimes the best way to understand a tool is to build the thing the tool would normally hide from you.

#Rust #SystemsProgramming #SoftwareEngineering

---

## Post 2 — Technical deep-dive (protocol design)

Why would you use TCP *and* UDP in the same app? 🤔

While building a stock-quote streamer in Rust, I split the protocol into two planes:

📞 **Control plane → TCP.** The client opens a TCP connection, sends a JSON `STREAM` request (its listen address + the tickers it wants), and gets a reliable Ok/Error reply. Reliability matters for the handshake.

⚡ **Data plane → UDP.** Once subscribed, the server fires quote batches at the client over UDP. Fire-and-forget, low latency — exactly what you want for a fast-moving stream where a dropped packet is fine but head-of-line blocking is not.

The tricky part: with UDP there's no connection to tell you a client disappeared. So I added a heartbeat — the client pings over UDP, the server records each client's last-seen `Instant`, and any client silent for 5s is automatically evicted from the broadcast and cleaned up.

Choosing the right transport per concern, instead of forcing everything through one, made the whole design click.

#Rust #NetworkProgramming #DistributedSystems #Backend

---

## Post 3 — Lessons learned / reflection

Every pet project teaches you something the tutorials skip. Here's what shipping a Rust streaming server taught me 👇

1️⃣ **Lock poisoning is real.** I started with `std::sync::RwLock` and hit poisoning headaches. Migrating to `parking_lot::RwLock` — and using `upgradable_read` for the eviction path — cleaned it up dramatically.

2️⃣ **`Instant` vs `SystemTime` matters.** For measuring "has this client gone quiet?", monotonic time (`Instant`) is the correct tool — wall-clock time can jump backwards.

3️⃣ **Graceful shutdown is a feature, not an afterthought.** A `signal-hook` handler flips an `AtomicBool`; every thread notices, unwinds, and joins cleanly. No orphaned threads, no half-written state.

4️⃣ **Backpressure forces honest design.** Bounded channels (capacity 1) meant I had to decide what happens when a consumer is slow — instead of pretending memory is infinite.

5️⃣ **Small projects deserve real tooling.** Typed errors with `thiserror`, structured logging with `tracing`, doctests in the library, thin LTO in release. It's a portfolio project, but the discipline transfers.

The roadmap: custom derive macros, `crossbeam`, and `socket2`. The learning never really stops. 🚀

#Rust #SoftwareCraftsmanship #LearningInPublic #Concurrency
