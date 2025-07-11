# SysPort

[![GitHub stars](https://img.shields.io/github/stars/ND-Zyth/sysport?style=social)](https://github.com/xiaohu/sysport/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/ND-Zyth/sysport?style=social)](https://github.com/xiaohu/sysport/forkers)

**SysPort** is a modern, cross-platform system monitor with a beautiful GUI, real-time metrics, plugin support, and advanced network monitoring.

## Features
- CPU, memory, disk, and network monitoring
- Customizable themes
- Plugin system (Lua)
- Alerts, notifications, export/import
- Cross-platform: Windows, macOS, Linux

## Quick Start
```sh
# Clone and build
cargo build --release
# Run
./target/release/sysport
```

## Minimal Plugin Example
Create a file in `plugins/lua/`:
```lua
function on_load()
    print("Plugin loaded!")
end
function hello()
    print("Hello from plugin!")
end
register_command("hello", hello)
```

## License
MIT 