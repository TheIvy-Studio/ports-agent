Name:           ports-agent
Version:        0.1.0
Release:        1%{?dist}
Summary:        Ports agent — native systemd node agent and portsctl CLI

License:        MIT
URL:            https://github.com/TheiVy-Studio/ports-agent
BuildRequires:  cargo
Requires:       nftables

%description
Ports agent is the native systemd node agent for the Ports control plane.
It manages nftables port-forward rules and reports telemetry, and ships the
portsctl CLI for enrollment and local node management.

%build
cargo build --release --locked

%install
install -Dm755 target/release/ports-agent %{buildroot}/usr/bin/ports-agent
install -Dm755 target/release/portsctl %{buildroot}/usr/bin/portsctl
install -Dm644 packaging/systemd/ports-agent.service %{buildroot}/usr/lib/systemd/system/ports-agent.service
install -dm755 %{buildroot}/etc/ports/config
install -dm750 %{buildroot}/etc/ports/keys

%files
/usr/bin/ports-agent
/usr/bin/portsctl
/usr/lib/systemd/system/ports-agent.service
%dir /etc/ports/config
%dir /etc/ports/keys

%post
if ! id ports >/dev/null 2>&1; then
    useradd --system --no-create-home --home-dir /var/lib/ports --shell /sbin/nologin ports || true
fi
mkdir -p /var/lib/ports /var/log/ports
%systemd_post ports-agent.service

%preun
%systemd_preun ports-agent.service

%postun
%systemd_postun_with_restart ports-agent.service
