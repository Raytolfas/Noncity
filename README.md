<div align="center">

<img src="https://github.com/MIXITOSIK/MIXITOSIK/blob/main/other_assets/banner-noncity.png?raw=true" alt="RMC Shaders Image Banner" width="512">

**Noncity - High performance Rust Minecraft proxy**

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Minecraft](https://img.shields.io/badge/Minecraft%20Java-1.20.5%20--%2026.3%2B-brightgreen.svg)](https://minecraft.wiki/)
[![Ko-fi](https://img.shields.io/badge/Support-Ko--fi-ff5e5b.svg)](https://ko-fi.com/mixitosik)

</div>

Noncity is an asynchronous Minecraft proxy written in Rust, engineered for low latency, minimal memory usage, and seamless multi-version server switching.

---

## Features

- **Multi-Version Protocol Support**: Compatible with modern Minecraft Java Edition releases (1.20.5 through 26.3+), adapting packet structures dynamically per client protocol.
- **Asynchronous Pipeline**: Built on Tokio with zero-copy buffer operations and dedicated client-backend frame forwarding.
- **In-Game Navigation**:
  - `/server [name]` — Switch backend servers on the fly with tab completion.
  - `/noncity` — Query proxy status.
- **Rhai Plugin Scripting**: Embedded lightweight scripting for hook-based authorization (`on_player_login`) and custom command handling (`on_command`).
- **Velocity Modern Forwarding**: Authenticated player data forwarding with HMAC-SHA256 signature verification.
- **Online & Offline Authentication**: Support for Mojang session verification as well as offline UUID derivation.

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (version 1.85 or newer)
- Git

### Building from Source

```bash
git clone https://github.com/Raytolfas/Noncity.git
cd Noncity
cargo build --release
```

The compiled executable will be located at `target/release/noncity` (or `target/release/noncity.exe` on Windows).

### Running

```bash
./target/release/noncity
```

On first startup, Noncity creates a default configuration file `config.toml` if one is not present.

---

## Configuration (`config.toml`)

```toml
[proxy]
bind_address = "0.0.0.0:25565"
motd = "§bNoncity §7- Modern Rust Minecraft Proxy"
max_players = 1000
online_mode = false
compression_threshold = 256
forwarding_mode = "velocity"
velocity_secret = "your_secret_here"

[servers.lobby]
address = "127.0.0.1:25566"

[servers.survival]
address = "127.0.0.1:25567"
```

---

## Support & Funding

If you find Noncity useful, you can support development on Ko-fi:

[![Ko-fi](https://img.shields.io/badge/Ko--fi-mixitosik-ff5e5b?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/mixitosik)

---

## License

Noncity is licensed under the [Apache License, Version 2.0](LICENSE).
