# ports-agent

Native, small, "dumb-by-design" node agent for the **Ports** project, plus the
`portsctl` CLI. Written in Rust. The agent only executes structured RPC commands
from `ports-backend` and reports telemetry — it holds no users, no UI and no
panel business logic.

## Workspace layout

```
crates/
  ports-common/        shared DTOs — RPC protocol envelope + agent config + paths
  ports-agent/         the agent (library + daemon), src/ split into:
    src/core/          system facts, telemetry, ed25519 keys, logging, version
    src/db/            local state store (applied rules) under /var/lib/ports
    src/firewall/      provider trait + nftables backend (plan / apply / backup)
    src/router/        dispatch: maps one RPC message to a handler
    src/transport/     reverse (outbound WebSocket) + stdio (SSH agent-rpc)
  portsctl/            CLI, src/ split into core/ (backend client, systemd) + commands/
packaging/             systemd unit, nfpm (deb/rpm/apk), pacman PKGBUILD, rpm spec, installer
```

`router::dispatch` is the single, transport-agnostic command handler. The daemon
feeds it messages from the reverse WebSocket; `portsctl agent-rpc` feeds it the
same messages over stdin/stdout (SSH mode). Identical protocol, identical logic.

## Connection modes

- **Reverse (mode B):** `ports-agent.service` dials out to
  `backend.websocket_url`, sends a signed `hello`, then serves RPC and pushes a
  telemetry snapshot every 15s. No inbound ports on the node.
- **Direct SSH (mode A):** no daemon runs. The backend SSHes in and runs the
  restricted `portsctl agent-rpc`, exchanging one JSON request/response per call.

## Files & directories

| Path | Purpose |
|------|---------|
| `/etc/ports/config/agent.yaml` | agent configuration (see `packaging/config/agent.example.yaml`) |
| `/etc/ports/keys/node.key` / `node.pub` | node Ed25519 identity (key is `0600`) |
| `/var/lib/ports/state.json` | applied port-forward rules + last backup |
| `/var/lib/ports/backups/` | nftables ruleset backups taken before apply |
| `/var/log/ports/` | log directory (managed by systemd `LogsDirectory`) |

The systemd unit declares `ConfigurationDirectory`/`StateDirectory`/`LogsDirectory`
so `/etc/ports`, `/var/lib/ports` and `/var/log/ports` are created and owned
automatically; package post-install creates `config/` + `keys/` and the `ports`
system user.

## portsctl

```
portsctl login [<backend-url> <enrollment-token>] [--mode ssh|reverse]
portsctl logout
portsctl status
portsctl config
portsctl node info | node rename <name>
portsctl agent start | stop | restart
portsctl check
portsctl detect-interfaces
portsctl firewall plan | apply | backup | restore <path>
portsctl logs [--lines N]
portsctl agent-rpc          # internal: restricted SSH command, JSON over stdin/stdout
```

`login` generates the node keypair, enrolls against
`POST /api/agent/enroll`, writes `/etc/ports/config/agent.yaml`, and (reverse
mode) enables + starts the service.

## Build

```bash
cargo build --release          # target/release/ports-agent, target/release/portsctl
cargo check --workspace
```

## Packaging (deb / rpm / pacman / apk)

`nfpm` builds Debian, RPM and Alpine packages from one spec; Arch uses a PKGBUILD:

```bash
make deb      # dist/ports-agent_<arch>.deb       (apt / dpkg)
make rpm      # dist/ports-agent_<arch>.rpm        (dnf / yum / rpm)
make apk      # dist/ports-agent_<arch>.apk        (apk)
make arch     # packaging/pacman -> *.pkg.tar.zst  (pacman -U)
make packages # deb + rpm + apk
```

Universal one-liner (downloads a prebuilt package for the distro, or builds from
source, then enrolls):

```bash
curl -fsSL https://ports.example.com/install-agent.sh | sudo bash -s -- \
  --backend https://ports.example.com --mode reverse --token ps_enroll_xxx
```

Or per-distro after the package is installed:

```bash
sudo apt install ./ports-agent_amd64.deb     # Debian/Ubuntu
sudo dnf install ./ports-agent_amd64.rpm      # Fedora/RHEL
sudo pacman -U ports-agent-*.pkg.tar.zst      # Arch
sudo apk add --allow-untrusted ports-agent_amd64.apk
sudo portsctl login https://ports.example.com ps_enroll_xxx
```

## Firewall model

nftables-first (iptables detection is reported as a fallback backend). All
managed rules live in their own table (`ports_agent` by default) across a
`prerouting` (DNAT) and `postrouting` (masquerade) chain, each rule tagged with
its `ruleId` comment. Existing system rules are never flushed. Apply always
takes a ruleset backup first when `backup_before_apply` is set, and the backend
requires a dry-run `plan` before any `apply`.
