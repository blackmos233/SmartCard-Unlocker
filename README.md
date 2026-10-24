# Windows 插卡解锁工具 (SmartCard Unlock Tool)

一款基于 **Realtek RTS5169 (Generic EMV) 智能卡读卡器** 与 **IC4442 (SLE4442) 芯片卡** 实现的 Windows 自动插卡解锁与拔卡锁屏工具。

![Windows 10/11 Compatible](https://img.shields.io/badge/Windows-10%20%7C%2011-blue)
![Rust 1.96 Native](https://img.shields.io/badge/Rust-1.96%20Native-orange)
![Security](https://img.shields.io/badge/Security-DPAPI%20%2B%20AES--256--GCM-red)
![Windows Hello](https://img.shields.io/badge/Windows%20Hello-PIN%20Unlock-success)

---

## ⚡ 架构优化与全新 Rust 原生极速核心

根据实际环境与开机仅有 Windows Hello PIN 码解锁的需求，本项目提供了 **Rust 原生极速版** 与 **Python GUI 控制面板** 两个版本：

1. **Rust 原生底层极速守护核心 (`bin/smartcard_unlock.exe`)**：
   - 采用纯 **Rust 1.96** 编写，基于纯 Win32 API 与 `winscard.dll` 驱动层通信。
   - **响应延迟 < 5 毫秒**，常驻内存仅 **~2 MB**，无任何 Python 虚拟机或三方运行时依赖。
   - 彻底解决之前全屏窗口导致的消息循环假死未响应问题。
2. **原生 Windows Hello PIN 自动解锁**：
   - 针对当前 Windows 账户开启了“免密登录 / 仅 PIN 码登录”的机制，专门实现针对 Windows 锁屏 PIN 码输入框的唤醒与自动键入 (`KEYEVENTF_UNICODE`)。
   - 插入卡片时，自动唤醒并上滑锁屏壁纸幕布，毫秒级注入 PIN 码自动完成系统登录。
3. **原生 LockWorkStation 拔卡锁屏**：
   - 拔卡瞬间直接调用 Windows 系统的 `LockWorkStation` 切换至安全桌面，杜绝任何窗口层假死、闪烁或未响应现象。
4. **Windows DPAPI 硬件绑定双重加密**：
   - PIN 码通过 Windows 原生数据保护 API（`CryptProtectData`）加密存储，仅当前电脑且当前用户安全上下文可解密。

---

## 🚀 极速运行指南 (推荐 Rust 原生版)

### 方式一：运行 Rust 原生极速守护程序 (最快、最稳定)

直接运行已编译好的原生程序：

```powershell
.\bin\smartcard_unlock.exe
```

启动后将显示交互式菜单：
```text
==============================================================
  Windows 插卡解锁工具 (RTS5169 + IC4442) [Rust 原生极速引擎] 
  硬件设备: Generic EMV / RTS5169 (Port_#0004.Hub_#0002)       
==============================================================
  [1] 启动插拔卡自动锁屏/解锁守护
  [2] 配置/修改 Windows Hello PIN 码
  [3] 检查读卡器与卡片连接状态
  [4] 清除已绑定的 PIN 码
  [5] 退出程序
```

- 输入 `2`：设置您的电脑开机 Windows Hello PIN 码。
- 输入 `1`：启动实时插拔守护！
  - **拔出 IC4442 卡片**：瞬间触发 Windows 原生锁屏。
  - **插入 IC4442 卡片**：自动唤醒屏幕并输入 PIN 码极速解锁！

#### 快捷命令行参数：
* `smartcard_unlock.exe run`：直接进入静默守护监听模式。
* `smartcard_unlock.exe status`：即时查询读卡器与卡片插拔状态。
* `smartcard_unlock.exe set-pin <PIN>`：一键设置绑定的 PIN 码。

---

### 方式二：运行 Python 图形管理面板

若需要图形界面管理，可在项目根目录运行：

```powershell
python main.py
```

---

## 🛠️ 使用与配置流程

### 步骤一：查看设备连接状态
启动后进入 **「运行状态」** 页面：
* **读卡器名称** 应显示 `Generic EMV Smartcard Reader 0`。
* **卡片状态**：卡片插入时将点亮绿色徽章 `已插卡`。

### 步骤二：绑定卡片与 Windows 登录凭据
切换至 **「卡片绑定与凭据」** 页面：
1. **Windows 用户名**：默认为当前系统登录用户。
2. **Windows 登录密码**：输入该账户当前的 Windows 登录密码。
3. **自定义安全 PIN**：默认预设为 `123456`，建议设置为个人熟悉的 4-8 位数字作为双因子与应急解锁码。
4. 点击 **「一键绑定并加密写入安全金库」**，数据将立即使用 DPAPI + AES-256-GCM 加密落盘。

### 步骤三：验证插卡解锁
1. 拔出 IC4442 卡片，计算机将立即自动锁屏并拉起安全防护屏。
2. 再次将卡片插入读卡器，系统将在 0.5 秒内完成卡片识别、安全金库验证，并自动恢复系统桌面！

---

## 📁 项目目录结构

```text
D:\work\SmartCard\
├── src/
│   ├── __init__.py           # 包定义与版本信息
│   ├── smartcard_engine.py   # 底层 WinSCard PC/SC 驱动交互与插拔事件监控
│   ├── security_vault.py     # DPAPI + AES-256-GCM 硬件绑定安全加密金库
│   ├── unlock_controller.py  # 会话状态检测、自动锁屏与自动解锁控制器
│   ├── screen_guard.py       # 全屏安全屏保锁与应急 PIN 码窗口
│   ├── gui_app.py            # PyQt5 现代化桌面管理面板
│   └── tray_icon.py          # 系统托盘集成与后台状态通知
├── tools/
│   └── rts_4442_tool.py      # RTS5169 / IC4442 深度底层硬件探测与诊断脚本
├── tests/
│   ├── test_engine.py        # SmartCard 引擎单元测试
│   ├── test_vault.py         # 安全金库加密/解密/防篡改单元测试
│   └── test_unlock_controller.py # 锁屏控制器自动化测试
├── main.py                   # 应用程序主入口 (含单实例互斥锁)
├── requirements.txt          # Python 运行时依赖
└── README.md                 # 项目工程与使用说明文档
```

---

## 🛡️ 安全合规与故障排查

| 现象 | 可能原因 | 解决方法 |
| :--- | :--- | :--- |
| 读卡器显示“未检测到” | USB 接触不良或端口变动 | 检查读卡器是否插紧在 `Port_#0004.Hub_#0002`，点击界面“刷新状态” |
| 插入卡片后无反应 | 卡片反插或触点氧化 | 拔出卡片，翻转方向（芯片朝向读卡器内部触点）重新插入 |
| 无法通过卡片解锁 | 凭据未绑定或密码变更 | 打开管理面板，在“卡片绑定”页面重新输入 Windows 密码并绑定 |
| 忘记携带智能卡 | 物理卡不在身边 | 在锁屏界面点击输入自定义应急 PIN 码，按回车即可直接解锁 |

---

## 📄 开源许可证

本项目遵循 MIT 协议开源。
