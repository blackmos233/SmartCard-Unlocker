# Windows 智能卡解锁系统 (SmartCard RTS5169 / IC4442 Auto Unlock)

基于 **Realtek RTS5169 (Generic EMV) 智能卡读卡器** 与 **IC4442 (SLE4442) 接触式芯片卡** 实现的 Windows 硬件拔卡秒锁与插卡原生解锁系统。

---

## ⚡ 系统核心架构

本项目已全面升级为原生底层系统架构，彻底移除 Python 运行时依赖与界面模拟按键，由两大原生核心组件协同工作：

1. **Windows 原生凭据提供程序 (Credential Provider COM DLL)**：
   - 路径：`bin/SmartCardCredentialProvider.dll`（注册至 Windows 系统 LSA）
   - 采用 C++ 实现微软官方 `ICredentialProvider` / `ICredentialProviderCredential2` 接口。
   - 物理插卡瞬间由系统级 Winlogon 直接唤醒，原生毫秒级认证放行，杜绝任何桌面注入限制、全屏假死或 UAC 拦截。
   - 内置硬件物理在位强校验（PC/SC WinSCard 实时防线），未插卡时彻底隐藏提交箭头并强制拦截，杜绝认证绕过。

2. **Windows 后台硬件监控系统服务 (SmartCardUnlock Service)**：
   - 路径：`bin/smartcard_unlock.exe`（基于 Rust 编写，常驻 SYSTEM 权限运行）
   - 内存占用仅 **~2 MB**，CPU 占用 **0%**。
   - 智能处理 USB 读卡器热插拔与设备断开自动重连，无需重启服务。
   - 拔出 IC4442 卡片或拔出读卡器 USB 时：**毫秒级触发 Windows 原生安全锁屏 (`LockWorkStation`)**。
   - 插入卡片时：向 Winlogon 发送硬件在位事件，触发原生秒级解锁。

3. **Windows DPAPI 硬件级安全密码库**：
   - 存储于 `C:\ProgramData\SmartCardUnlock\vault.dat`。
   - 采用 Windows 原生数据保护接口（`CryptProtectData`），仅本机安全凭据上下文可解密。

---

## 📁 项目目录结构

```text
D:\work\SmartCard\
├── bin\                               # 编译完成的原生二进制与 DLL
│   ├── SmartCardCredentialProvider.dll # Windows 原生凭据提供程序 COM DLL
│   └── smartcard_unlock.exe           # Rust 核心系统服务与管理命令行工具
├── credential_provider\               # C++ 原生凭据提供程序源码 (MinGW GCC)
│   ├── CClassFactory.cpp / .h         # COM 类工厂
│   ├── CSmartCardProvider.cpp / .h    # 凭据提供程序管理器 & PC/SC 状态监控
│   ├── CSmartCardCredential.cpp / .h  # 凭据磁贴实现 & 硬件物理防线 & 序列化认证
│   ├── dllmain.cpp                    # COM DLL 注册与导出
│   └── build_credprov.bat             # 编译脚本
├── rust-core\                         # Rust 系统服务源码 (Cargo)
│   ├── src\
│   │   ├── main.rs                    # 命令行管理入口 & 控制台前台模式
│   │   ├── service.rs                 # Windows NT Service (SYSTEM 权限守护)
│   │   ├── scard.rs                   # PC/SC WinSCard 底层通信与热插拔重连
│   │   ├── lock.rs                    # 原生锁屏与会话状态管理
│   │   └── vault.rs                   # DPAPI 凭据加解密
│   └── Cargo.toml
├── tools\                             # 硬件诊断脚本
│   └── rts_4442_tool.py               # SLE4442 EEPROM 低级诊断工具
├── set_password.bat                   # 快捷更新 Windows 解锁密码
├── install_service.bat                # 一键安装后台系统服务并注册凭据提供程序
├── uninstall_service.bat              # 一键卸载系统服务
├── update_service.bat                 # 一键更新服务与凭据提供程序二进制
└── run_console.bat                    # 前台控制台调试模式
```

---

## 🚀 常用操作指南

### 1. 配置或修改 Windows 解锁密码
双击运行根目录下的：
> 👉 **`set_password.bat`**
根据提示输入最新的 Windows 登录密码并按回车即可。

### 2. 检查系统与硬件状态
在终端运行：
```powershell
.\bin\smartcard_unlock.exe status
```

### 3. 安装或更新服务
- 安装服务：管理员运行 `install_service.bat`。
- 更新二进制：修改代码后运行 `update_service.bat` 即可全自动完成重新编译与热替换。
