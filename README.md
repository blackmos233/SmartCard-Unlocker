# Windows 插卡解锁工具 (SmartCard Unlock Tool)

一款基于 **Realtek RTS5169 (Generic EMV) 智能卡读卡器** 与 **IC4442 (SLE4442) 芯片卡** 实现的 Windows 自动插卡解锁与拔卡锁屏工具。

![Windows 10/11 Compatible](https://img.shields.io/badge/Windows-10%20%7C%2011-blue)
![Python 3.10+](https://img.shields.io/badge/Python-3.10%2B-green)
![PyQt5](https://img.shields.io/badge/GUI-PyQt5-informational)
![Security](https://img.shields.io/badge/Security-DPAPI%20%2B%20AES--256--GCM-red)

---

## 📖 核心特性与架构

本工具专为配合硬件读卡器与接触式 IC 卡打造，在实现插卡即解锁、拔卡即锁屏的同时，遵循企业级安全设计规范：

1. **实时插拔感应与自动响应**
   - **插卡解锁**：当合法绑定的 IC4442 插入读卡器时，毫秒级响应并安全解除锁定，恢复 Windows 工作桌面。
   - **拔卡锁屏**：离开电脑拔出卡片瞬间，立即调用 Windows 原生 `LockWorkStation` 锁定工作站，同时拉起全屏安全防护屏障（Screen Guard）。
2. **多层防御型安全金库 (Cryptographic Vault)**
   - **拒绝明文**：绝不在磁盘、注册表或卡片上明文存放 Windows 密码。
   - **Windows DPAPI 双重加密**：第一层采用 Windows 用户级数据保护 API (`CryptProtectData`)，确保凭据只能在当前登录用户的安全上下文中被读取。
   - **AES-256-GCM 认证加密**：第二层使用 256 位 AES-GCM 算法，提供机密性与防篡改完整性校验（16 字节认证 Tag）。
   - **PBKDF2 强密钥派生**：通过 100,000 次 HMAC-SHA256 迭代，融合 **硬件唯一识别码 (Machine GUID)**、**读卡器物理端口标识** 与 **用户安全 PIN 码** 派生解密密钥，杜绝跨机器盗用或脱机字典攻击。
   - **防暴力破解保护**：连续 5 次验证失败自动触发 60 秒冷却锁定。
3. **双重防护模式**
   - **Windows 原生锁屏联动**：直接联动系统登录锁定，支持自动唤醒屏幕幕布。
   - **Screen Guard 全屏安全防线**：在拔卡时自动展示全屏毛玻璃锁定界面，拦截非常规按键与窗口旁路，提供应急 PIN 码备用解锁途径。
4. **系统托盘与静默开机自启**
   - 支持最小化至 Windows 系统任务栏托盘持续守护。
   - 一键配置开机自动启动并在后台静默运行。

---

## 🔌 硬件环境与配置

* **读卡器控制器**：Realtek RTS5169 / Alcorlink AK9563 (Generic EMV Smartcard Reader)
* **硬件设备位置**：`Port_#0004.Hub_#0002`
* **智能卡规格**：IC4442 / SLE4442 接触式加密存储卡 (256 字节 EEPROM + 3 字节 PSC 安全码)
* **系统环境**：Windows 10 / Windows 11 64 位

> [!TIP]
> **插卡方向说明**：
> 接触式智能卡的芯片引脚位于卡片单侧。若插入后读卡器指示灯未闪烁或处于 Mute 状态，请确认卡片插入方向是否与读卡器金属触点贴合（芯片触点朝上或朝下）。

---

## 🚀 快速开始

### 1. 环境准备

确保已安装 Python 3.10 及以上环境，并安装依赖包：

```bash
pip install -r requirements.txt
```

### 2. 启动程序

在项目根目录下直接运行：

```bash
python main.py
```

若希望后台静默启动（如加入开机自启）：

```bash
python main.py --silent
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
