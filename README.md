# Claude Samurai - Configure your Claude Code without pain

<div align="center">

**Visual configuration manager for Claude Code and MCP**

<img src="src-tauri/icons/icon.png" alt="Claude Samurai icon" width="160" height="160" />


</div>

## 📸 Screenshots

### Configuration Management

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.54.51](screenshot/Screenshot%202026-02-10%20at%2019.54.51.png)

<details>
<summary><strong>All Screenshot</strong></summary>

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.55.36](screenshot/Screenshot%202026-02-10%20at%2019.55.36.png)

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.55.58](screenshot/Screenshot%202026-02-10%20at%2019.55.58.png)

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.56.16](screenshot/Screenshot%202026-02-10%20at%2019.56.16.png)

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.56.27](screenshot/Screenshot%202026-02-10%20at%2019.56.27.png)

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.56.32](screenshot/Screenshot%202026-02-10%20at%2019.56.32.png)

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.56.46](screenshot/Screenshot%202026-02-10%20at%2019.56.46.png)

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.56.53](screenshot/Screenshot%202026-02-10%20at%2019.56.53.png)

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.56.57](screenshot/Screenshot%202026-02-10%20at%2019.56.57.png)

- **Title**: _TODO: add title_
- **Description**: _TODO: add description_

![Screenshot 2026-02-10 at 19.57.04](screenshot/Screenshot%202026-02-10%20at%2019.57.04.png)

</details>

## ✨ Features

- **Multi-configuration support**: Switch between multiple Claude Code configurations effortlessly
- **MCP server management**: Configure and manage Model Context Protocol (MCP) servers
- **Agent management**: Manage Claude Code agents and their settings
- **Global commands**: Configure and organize global commands
- **Skills**: Define and manage reusable skills for Claude Code and MCP servers
- **Plugins**: Configure and control external plugins and integrations
- **Marketplace**: Discover, install, and manage community skills, plugins, and packs
- **Commands**: Create and manage reusable command workflows for common tasks
- **Hooks**: Attach automation hooks to Claude Code and MCP events
- **Security packs**: Ready‑to‑use bundles of security agents, commands, skills, and configurations
- **CLAUDE.md integration**: Read and write global CLAUDE.md memory files
- **Usage analytics**: Track and analyze your Claude Code usage

## 🚀 Quick Start

<!-- ## 🐳 Build with Docker

You can create a Linux (x86_64) release build inside a Docker container using the provided `Dockerfile`:

```bash
# Build the image (Linux x86_64 target by default)
docker build -t ccmate-builder .

# Create a temporary container from the image
docker create --name ccmate-out ccmate-builder

# Copy artifacts (AppImage / deb / other bundles) to ./dist
docker cp ccmate-out:/artifacts ./dist

# Clean up the temporary container
docker rm ccmate-out
``` -->

### Build on macOS

First, install the toolchain:

```bash
# Linux/MacOs
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install pnpm (JavaScript package manager)
## POSIX systems
curl -fsSL https://get.pnpm.io/install.sh | env PNPM_VERSION=10.28.2 sh -
```

Then run the build:

```bash
pnpm install
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm tauri build
```

Artifacts will be under:

- `src-tauri/target/*/release/bundle/`

## 📄 License

This project is licensed under the **GNU Affero General Public License v3.0**.

See the [LICENSE](LICENSE) file for details.

## Attribution

Claude Samurai is based on the open‑source project [CC Mate](https://github.com/djyde/ccmate) created by djyde.  
Original work is licensed under the GNU Affero General Public License v3.0 (AGPLv3), and this project continues under the same license.