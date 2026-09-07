# 代码来源与修改说明

## 项目来源

本仓库是 **Clash Verge Rev 的非官方修改版**，代码基础来自：

- 上游项目：<https://github.com/Clash-Verge-rev/clash-verge-rev>
- 基础版本：`v2.5.2`
- 上游发布页：<https://github.com/Clash-Verge-rev/clash-verge-rev/releases/tag/v2.5.2>
- 原始项目说明：Clash Verge 的延续版本，使用 Rust、Tauri 和 Web 前端构建。

当前工作目录来自上游源码快照，不包含原始 `.git` 历史，因此本仓库的首次提交代表“上游 v2.5.2 源码快照 + 下述本地修改”，并不重写或声称拥有上游提交历史。

本项目及修改继续遵循仓库中的 [GNU GPL v3 许可证](./LICENSE)。Clash Verge Rev、Clash Verge、mihomo、Tauri 及其他依赖的名称与商标归各自权利人所有。本仓库及其 Release **不是 Clash Verge Rev 官方发布**。

## 本次修改

本次修改针对 macOS 首页开启“系统代理”时出现以下错误的问题：

```text
admin privileges required to modify system proxy
```

主要变更如下：

1. 当普通权限无法修改 macOS 系统代理时，使用系统标准管理员授权窗口请求权限。
2. 扩展 Clash Verge Service IPC，新增系统代理配置接口；Service 安装或升级并授权成功后，应用优先通过常驻特权 Service 修改代理，避免每次切换都要求输入密码。
3. Service 不可用、版本过旧或接口不可用时，保留管理员授权窗口作为降级路径。
4. 使用 SystemConfiguration 获取当前活动网络服务，并通过参数化的 `networksetup` 调用设置 HTTP、HTTPS、SOCKS、PAC 和 bypass 配置。
5. 对传入 Service 的网络服务名、主机、PAC URL 和 bypass 列表进行长度及控制字符校验，避免把未经验证的数据交给特权命令。
6. 将 Unix IPC socket 权限由 `0777` 收紧为 `0770`。
7. 修复系统代理副作用执行失败时，配置草稿未正确丢弃的问题。
8. 将本地 `clash_verge_service_ipc` 版本更新为 `2.3.4`，使旧 Service 能被识别为需要升级。

## 授权行为

- 首次安装或升级特权 Service 时，macOS 会要求管理员授权。
- Service 正常运行后，开启、关闭或更新系统代理不应再次要求输入管理员密码。
- 若未安装 Service，应用会在需要修改系统代理时弹出 macOS 管理员授权窗口。
- 程序不会保存、记录或自行处理管理员密码；“记住授权”通过 macOS LaunchDaemon/特权 Service 实现。

## 构建与验证

本地验证环境为 Apple Silicon macOS，使用 Rust 1.95.0、pnpm 和 Tauri 2。

已完成：

- Web 前端 TypeScript 检查和 Vite production build。
- Rust 应用及特权 Service 编译。
- 应用与 Service 的 Clippy 严格检查（`-D warnings`）。
- macOS shell 参数和 AppleScript 字符串转义单元测试。
- ARM64 `.app` 和 `.dmg` 构建。
- DMG `hdiutil verify` 完整性检查。
- DMG 内应用的 ad-hoc 代码签名检查。

发布文件：

```text
Clash Verge Rev Privilege Test_2.5.2_aarch64.dmg
SHA-256: 5c60c6e8c5836a9e3e93f9a68ab868e6be68c9e642b66b7e02a24d9ac8c5d8c1
```

该构建使用独立应用名称和 Bundle ID：

```text
名称：Clash Verge Rev Privilege Test
Bundle ID：io.github.clash-verge-rev.clash-verge-rev.privilege-test
```

应用使用本地 ad-hoc 签名，没有 Apple Developer ID 公证。首次运行时可能需要在“系统设置 → 隐私与安全性”中允许打开。

## 特权 Service 注意事项

虽然测试应用本身使用独立 Bundle ID，但特权 Service 沿用 Clash Verge Rev 的系统级 Service 路径和 LaunchDaemon 标识。安装或升级该 Service 会影响同一台 Mac 上正式版 Clash Verge Rev 对 Service 版本的判断。测试前建议退出正式版应用，并了解如何在应用设置中卸载或重新安装 Service。
