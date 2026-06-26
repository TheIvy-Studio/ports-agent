<h1 align="center">ports-agent</h1>

<h4 align="center">Native, small, dumb-by-design node agent for the Ports project, plus the portsctl CLI. It executes structured RPC commands from the backend and reports telemetry — no users, no UI, no panel logic.</h4>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge&logo=opensourceinitiative&logoColor=FFFFFF" alt="License"></a>
  <img src="https://img.shields.io/badge/Rust-2021-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Packaging-deb%20%7C%20rpm%20%7C%20apk%20%7C%20pacman-orange?style=for-the-badge&logo=linux&logoColor=FCC624" alt="Packaging">
  <img src="https://img.shields.io/badge/Platform-Linux-lightgrey?style=for-the-badge&logo=linux&logoColor=FCC624" alt="Platform">
</p>

---

**ports-agent** is the per-node executor for [Ports](https://github.com/IMDelewer/ports). The backend speaks one RPC envelope to it over two interchangeable transports; the agent applies firewall, DHCP, proxy, certificate and DNS changes locally and never makes policy decisions of its own.

---

## 🔌 Connection modes

- **Reverse (mode B):** `ports-agent.service` dials out to the backend WebSocket, sends a signed `hello`, serves RPC and pushes telemetry every 15s. No inbound ports on the node.
- **Direct SSH (mode A):** no daemon; the backend SSHes in and runs the restricted `portsctl agent-rpc`, one JSON request/response per call.
- **Tailscale SSH:** same as SSH, reached over the node's Tailscale IP.

`router::dispatch` is the single, transport-agnostic command handler — identical protocol, identical logic on every transport.

---

## 🧩 Handlers

| Domain | Commands |
| :--- | :--- |
| **Firewall** | `firewall.plan` / `apply` / `delete` / `backup` / `restore` (nftables) |
| **Discovery** | `discovery.scan` (system, network, ports, docker, tailscale, firewall) |
| **DHCP** | `dhcp.plan` / `dhcp.apply` (dnsmasq and Kea) |
| **Proxy** | `haproxy.validate` / `haproxy.reload` |
| **Certificates** | `cert.issue` / `cert.renew` (lego) |
| **DNS** | `dns.plan` / `dns.apply` (CoreDNS) |
| **Mesh** | `tailscale.status` |
| **Backup** | `backup.create` / `backup.restore` |
| **Traffic** | `conntrack.list`, telemetry snapshots |

---

## 🖥️ portsctl

```
portsctl login [<backend-url> <enrollment-token>] [--mode ssh|reverse|tailscale] [--tailscale-ip <ip>]
portsctl logout | status | config | check
portsctl node info | node rename <name>
portsctl agent start | stop | restart
portsctl detect-interfaces
portsctl firewall plan | apply | backup | restore <path>
portsctl logs [--lines N]
portsctl agent-rpc          # internal: restricted SSH command, JSON over stdin/stdout
```

`login` generates the node Ed25519 keypair, enrolls against `POST /api/agent/enroll`, writes `/etc/ports/config/agent.yaml`, and (reverse mode) enables and starts the service.

---

## 📦 Build & package

```bash
cargo build --release
cargo check --workspace

make deb        # dist/ports-agent_<arch>.deb
make rpm        # dist/ports-agent_<arch>.rpm
make apk        # dist/ports-agent_<arch>.apk
make arch       # packaging/pacman -> *.pkg.tar.zst
make packages   # deb + rpm + apk
```

Universal installer:

```bash
curl -fsSL https://ports.example.com/install-agent.sh | sudo bash -s -- \
  --backend https://ports.example.com --mode reverse --token ps_enroll_xxx
```

---

## ⚙️ Files & directories

| Path | Purpose |
| :--- | :--- |
| `/etc/ports/config/agent.yaml` | agent configuration |
| `/etc/ports/keys/node.key` · `node.pub` | node Ed25519 identity (`0600`) |
| `/var/lib/ports/state.json` | applied rules + last backup |
| `/var/lib/ports/backups/` | config/ruleset backups taken before apply |
| `/var/log/ports/` | logs (systemd `LogsDirectory`) |

---

## 🗂 Structure

```
ports-agent/
├── crates/
│   ├── ports-common/       ← shared RPC protocol envelope, config, paths
│   ├── ports-agent/        ← daemon + library
│   │   └── src/
│   │       ├── core/       ← system facts, telemetry, keys, logging
│   │       ├── db/         ← local state store
│   │       ├── firewall/   ← nftables provider (plan/apply/backup)
│   │       ├── dhcp/       ← dnsmasq / Kea
│   │       ├── haproxy/    ← config validate / reload
│   │       ├── acme/       ← lego issue / renew
│   │       ├── coredns/    ← managed zones
│   │       ├── tailscale/  ← mesh status
│   │       ├── backup/     ← config backup / restore
│   │       ├── discovery/  ← read-only system scan
│   │       ├── traffic/    ← conntrack listing
│   │       ├── transport/  ← reverse WebSocket + stdio (SSH)
│   │       └── router/     ← single dispatch handler
│   └── portsctl/           ← CLI (core/ + commands/)
├── packaging/              ← systemd unit, nfpm, pacman, rpm, installer
├── Makefile
└── README.md
```

---

## 🛡️ Firewall model

nftables-first. All managed rules live in their own table (`ports_agent` by default) across `prerouting` (DNAT) and `postrouting` (masquerade) chains, each rule tagged with its `ruleId`. System rules are never flushed. Apply takes a ruleset backup first, and the backend requires a dry-run `plan` before any `apply`.

---

<p align="center"><sub><a href="LICENSE">MIT</a> © Ports</sub></p>
