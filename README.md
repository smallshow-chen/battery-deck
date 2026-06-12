# Battery Toolkit

macOS 电池管理工具，适用于 Apple Silicon Mac。通过 SMC 直接控制充电行为，提供可视化仪表盘和系统托盘菜单。

## 功能

- **充电策略**：充至满电 / 充至上限 / 恢复限充 / 停止充电 / 禁用适配器
- **充电限制**：可调节最低电量和最高电量阈值
- **实时监控**：温度、功率、电压、电流、充电状态
- **电池健康**：循环次数、电池寿命百分比、设计/实际/当前容量
- **设备信息**：机型、芯片、内存、首次设置日期
- **充电器信息**：适配器名称、功率、输出电压/电流
- **系统托盘**：右键菜单快捷操作，左键打开主窗口
- **特权服务**：以 root 权限运行的 helper 守护进程，通过 Unix Socket 与主应用通信
- **i18n**：自动检测系统语言，支持中文/英文
- **暗色模式**：跟随系统外观自动切换

## 技术栈

| 层级 | 技术 |
|------|------|
| 框架 | Tauri v2 |
| 后端 | Rust（`tauri`、`serde`、`tokio`、`regex`） |
| 前端 | Vanilla HTML + CSS + JavaScript |
| SMC 通信 | IOKit FFI（`core-foundation`、`mach2`） |
| 服务管理 | macOS LaunchDaemon / LaunchAgent |

## 项目结构

```
battery-toolkit-tauri/
├── src/                          # 前端
│   ├── index.html                # 主页面（仪表盘布局）
│   ├── main.js                   # 前端逻辑（Tauri invoke、轮询、DOM 更新）
│   ├── i18n.js                   # 国际化（中/英自动检测）
│   └── styles.css                # 样式（玻璃态设计、暗色模式、响应式）
├── src-tauri/                    # 后端
│   ├── src/
│   │   ├── main.rs               # 入口，调用 lib::run()
│   │   ├── lib.rs                # Tauri 命令注册、托盘菜单、窗口管理、电池轮询
│   │   ├── battery.rs            # 电池数据模型、ioreg/pmset/system_profiler 解析
│   │   ├── helper.rs             # root 守护进程（Unix Socket、SMC 控制、状态持久化）
│   │   ├── service.rs            # 服务生命周期管理（安装/启动/停止/通信）
│   │   ├── smc.rs                # Apple SMC IOKit FFI（读写充电控制寄存器）
│   │   └── bin/
│   │       └── battery-helper.rs # helper 二进制入口
│   ├── Cargo.toml
│   └── tauri.conf.json           # Tauri 配置（窗口、权限、打包）
├── scripts/
│   ├── restart-dev.sh            # 开发启动脚本（支持 --reinstall-root-helper）
│   └── package-release.sh        # 打包发布脚本（生成 .dmg 和 .zip）
└── package.json
```

## 快速开始

### 前置依赖

- macOS（Apple Silicon）
- [Rust](https://rustup.rs/)（stable）
- [Node.js](https://nodejs.org/)（v18+）

### 开发

```bash
cd battery-toolkit-tauri
npm install
./scripts/restart-dev.sh
```

如果需要重新安装 root helper 服务：

```bash
./scripts/restart-dev.sh --reinstall-root-helper
```

### 构建发布包

```bash
./scripts/package-release.sh
```

产物输出到 `release-artifacts/`：
- `MyBatteryManager.app` — macOS 应用包
- `MyBatteryManager_x.x.x_arm64.dmg` — 安装镜像
- `MyBatteryManager_x.x.x_arm64.zip` — 压缩包

## 架构概览

```
┌─────────────────────────────────────────────────┐
│  Tauri 前端（HTML/CSS/JS）                       │
│  invoke() ←→ Tauri Command                      │
└──────────────────────┬──────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────┐
│  lib.rs — Tauri 后端（19 个命令）                │
│  电池轮询 · 托盘菜单 · 窗口管理                  │
└──────────────────────┬──────────────────────────┘
                       │ Unix Socket (JSON)
┌──────────────────────▼──────────────────────────┐
│  battery-helper（root 守护进程）                 │
│  SMC 读写 · 充电控制 · 状态持久化                │
└─────────────────────────────────────────────────┘
```

### Tauri 命令列表

| 命令 | 说明 |
|------|------|
| `get_battery_state` | 获取电池综合状态 |
| `get_battery_realtime` | 获取实时数据（温度/功率/电压/电流） |
| `get_battery_health` | 获取电池健康信息 |
| `get_charger_info` | 获取充电器信息 |
| `get_system_info` | 获取设备信息（机型/芯片/内存） |
| `get_settings` / `set_settings` | 读取/保存设置 |
| `charge_to_full` | 充至满电 |
| `charge_to_limit` | 充至上限 |
| `disable_charging_cmd` | 停止充电 |
| `disable_adapter_cmd` / `enable_adapter_cmd` | 禁用/启用适配器 |
| `reset_charge_mode` | 恢复标准限充 |
| `get_service_status` | 获取服务状态 |
| `install_service` | 安装特权服务 |
| `start_service` / `stop_service` | 启动/停止服务 |
| `get_service_logs` | 获取服务日志 |
| `is_supported` | 检查设备支持 |

### Helper 通信协议

主应用与 helper 通过 Unix Socket 交换 JSON：

**请求**
```json
{
  "id": "uuid",
  "command": "charge_to_full",
  "payload": null
}
```

**响应**
```json
{
  "id": "uuid",
  "ok": true,
  "data": { "mode": "ToFull", "chargingDisabled": false, ... },
  "error": null
}
```

## CSS 架构

样式采用玻璃态（glassmorphism）设计，结构清晰：

| 区块 | 说明 |
|------|------|
| Variables | CSS 自定义属性（颜色、圆角、阴影、过渡） |
| Utility classes | `.card`（卡片背景）、`.label-caps`（大写标签） |
| App shell / Titlebar | 应用外壳和标题栏 |
| Overview | 电池状态面板（设备信息、电池图标、健康数据） |
| Service / Controls | 服务管理、操作按钮、充电限制滑块 |
| Dark mode | `@media (prefers-color-scheme: dark)` 全面适配 |
| Responsive | 920px / 760px 断点响应式布局 |

## 许可

[GPL v3](LICENSE)

## 致谢

- [Battery-Toolkit](https://github.com/mhaeuser/Battery-Toolkit) — 项目灵感来源，原项目使用 Swift 实现
