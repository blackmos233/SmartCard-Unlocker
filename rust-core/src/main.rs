mod scard;
mod lock;
mod vault;
mod service;

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use scard::SmartCardEngine;
use vault::SecurityVault;

fn print_banner() {
    println!("==============================================================");
    println!("  Windows 插卡解锁工具 (RTS5169 + IC4442) [Rust 原生极速引擎] ");
    println!("  硬件设备: Generic EMV / RTS5169 (Port_#0004.Hub_#0002)       ");
    println!("==============================================================");
}

fn run_daemon(engine: &SmartCardEngine, vault: &SecurityVault) {
    lock::enable_privilege("SeDebugPrivilege");
    lock::enable_privilege("SeImpersonatePrivilege");

    if !vault.is_enrolled() {
        println!("\n[提示] 当前尚未配置 Windows Hello PIN 码！");
        print!("请输入您电脑的 Windows Hello PIN 码进行加密绑定: ");
        io::stdout().flush().unwrap();
        let mut pin = String::new();
        io::stdin().read_line(&mut pin).unwrap();
        let pin = pin.trim();
        if pin.is_empty() {
            println!("[错误] PIN 码不能为空！");
            return;
        }
        if vault.save_pin(pin) {
            println!("[OK] Windows Hello PIN 码已通过 Windows DPAPI 安全加密存储！");
        } else {
            println!("[错误] 凭据加密失败！");
            return;
        }
    }

    if let None = vault.load_pin() {
        println!("[错误] 无法解密 PIN 码，请重新配置！");
        return;
    }

    println!("\n[OK] 智能卡安全守护已启动！(按 Ctrl+C 可停止)");
    println!("     -> 拔出 IC4442 卡片: 毫秒级触发 Windows 原生锁屏");
    println!("     -> 插入 IC4442 卡片: 系统原生凭据提供程序 (Credential Provider) 极速认证解锁\n");

    let mut last_present: Option<bool> = None;

    loop {
        let readers = engine.list_readers();
        if readers.is_empty() {
            println!("[警告] 未检测到 RTS5169 读卡器，请检查 USB 连接...");
            thread::sleep(Duration::from_millis(1000));
            continue;
        }

        let target_reader = readers
            .iter()
            .find(|r| {
                let lower = r.to_lowercase();
                lower.contains("emv") || lower.contains("rts") || lower.contains("realtek")
            })
            .unwrap_or(&readers[0]);

        let (present, _event_state) = engine.get_card_status(target_reader);

        match last_present {
            None => {
                last_present = Some(present);
                if present {
                    println!("[状态] 当前读卡器中已检测到 IC4442 卡片。");
                } else {
                    println!("[状态] 当前读卡器为空，等待插卡...");
                }
            }
            Some(prev) => {
                if prev != present {
                    last_present = Some(present);
                    let session_id = lock::get_active_session_id();

                    if present {
                        println!("\n💳 [事件] 检测到 IC4442 卡片插入！(会话 ID: {})", session_id);
                        if lock::is_session_locked(session_id) {
                            println!("   -> 当前系统处于锁定状态，由 Windows 原生凭据提供程序 (Credential Provider) 处理认证解锁。");
                        } else {
                            println!("   -> 当前系统未锁定，保持正常使用。");
                        }
                    } else {
                        println!("\n🔒 [事件] 检测到 IC4442 卡片已拔出！");
                        println!("   -> 立即执行 Windows 原生锁屏...");
                        let res = lock::lock_session(session_id);
                        if res {
                            println!("   -> [成功] 系统已安全锁定！");
                        } else {
                            println!("   -> [提示] 锁屏调用已触发。");
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn check_status(engine: &SmartCardEngine, vault: &SecurityVault) {
    println!("\n--- 硬件与安全状态诊断 ---");
    let readers = engine.list_readers();
    println!("1. 发现的智能卡读卡器 ({} 个):", readers.len());
    for r in &readers {
        println!("   -> {}", r);
    }

    if let Some(target) = readers.first() {
        let (present, state) = engine.get_card_status(target);
        println!("2. 卡片物理状态: {}", if present { "已插入 (PRESENT)" } else { "未插卡 (EMPTY)" });
        println!("   状态标志字: 0x{:08X}", state);
    }

    println!("3. Windows Hello PIN 绑定状态: {}", if vault.is_enrolled() { "已加密绑定" } else { "未绑定" });

    let session_id = lock::get_active_session_id();
    let locked = lock::is_session_locked(session_id);
    println!("4. Windows 会话锁定状态: {} (会话 ID: {})", if locked { "已锁定" } else { "活动中" }, session_id);

    let (installed, running) = service::is_service_installed_and_running();
    println!("5. Windows 后台系统服务状态: {}", 
        if !installed {
            "未安装"
        } else if running {
            "已安装并正在运行 (SYSTEM 权限)"
        } else {
            "已安装但已停止"
        }
    );

    let credprov_registered = service::is_credential_provider_registered();
    println!("6. Windows 原生凭据提供程序状态: {}",
        if credprov_registered {
            "已注册到系统 (CLSID: {7C3B984A-94B6-4E87-8A33-8FE58D6E9A42})"
        } else {
            "未注册 (可通过菜单 [7] 进行注册)"
        }
    );
    println!("--------------------------\n");
}

fn set_pin(vault: &SecurityVault) {
    print!("\n请输入要绑定的 Windows Hello PIN 码: ");
    io::stdout().flush().unwrap();
    let mut pin = String::new();
    io::stdin().read_line(&mut pin).unwrap();
    let pin = pin.trim();

    if pin.is_empty() {
        println!("[错误] PIN 码不能为空！");
        return;
    }

    if vault.save_pin(pin) {
        println!("[OK] Windows Hello PIN 码保存成功（已使用 DPAPI 机器级双重加密）！\n");
    } else {
        println!("[错误] 保存 PIN 码失败！\n");
    }
}

fn clear_pin(vault: &SecurityVault) {
    if vault.clear() {
        println!("\n[OK] 已清除绑定的 PIN 码！\n");
    } else {
        println!("\n[错误] 清除失败！\n");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "service-run" => {
                service::run_service();
                return;
            }
            "inject-pin" => {
                if args.len() > 2 {
                    lock::inject_pin_on_current_desktop(&args[2]);
                }
                return;
            }
            "lock-now" => {
                let session_id = lock::get_active_session_id();
                lock::lock_session(session_id);
                return;
            }
            "install-service" => {
                match service::install_service() {
                    Ok(_) => println!("[OK] Windows 插卡解锁服务安装成功并已启动！"),
                    Err(e) => eprintln!("[错误] 安装服务失败: {}", e),
                }
                return;
            }
            "uninstall-service" => {
                match service::uninstall_service() {
                    Ok(_) => println!("[OK] Windows 插卡解锁服务已卸载！"),
                    Err(e) => eprintln!("[错误] 卸载服务失败: {}", e),
                }
                return;
            }
            "start-service" => {
                match service::start_service() {
                    Ok(_) => println!("[OK] 服务已启动！"),
                    Err(e) => eprintln!("[错误] 启动服务失败: {}", e),
                }
                return;
            }
            "stop-service" => {
                match service::stop_service() {
                    Ok(_) => println!("[OK] 服务已停止！"),
                    Err(e) => eprintln!("[错误] 停止服务失败: {}", e),
                }
                return;
            }
            "run" | "daemon" => {
                let engine = match SmartCardEngine::new() {
                    Ok(e) => e,
                    Err(err) => {
                        eprintln!("[严重错误] 无法初始化 Windows PC/SC 智能卡服务 (代码: 0x{:08X})", err);
                        return;
                    }
                };
                let vault = SecurityVault::new();
                run_daemon(&engine, &vault);
                return;
            }
            "status" => {
                let engine = match SmartCardEngine::new() {
                    Ok(e) => e,
                    Err(err) => {
                        eprintln!("[严重错误] 无法初始化 Windows PC/SC 智能卡服务 (代码: 0x{:08X})", err);
                        return;
                    }
                };
                let vault = SecurityVault::new();
                check_status(&engine, &vault);
                return;
            }
            "set-pin" => {
                let vault = SecurityVault::new();
                if args.len() > 2 {
                    if vault.save_pin(args[2].trim()) {
                        println!("[OK] PIN 码设置成功！");
                    } else {
                        println!("[错误] PIN 码保存失败！");
                    }
                } else {
                    set_pin(&vault);
                }
                return;
            }
            "clear" => {
                let vault = SecurityVault::new();
                clear_pin(&vault);
                return;
            }
            "register-credprov" => {
                if service::register_credential_provider_dll() {
                    println!("[OK] Windows 原生凭据提供程序 (Credential Provider) 注册成功！");
                } else {
                    println!("[错误] 凭据提供程序注册失败，请确保以管理员权限或系统服务运行！");
                }
                return;
            }
            _ => {}
        }
    }

    let engine = match SmartCardEngine::new() {
        Ok(e) => e,
        Err(err) => {
            eprintln!("[严重错误] 无法初始化 Windows PC/SC 智能卡服务 (代码: 0x{:08X})", err);
            return;
        }
    };
    let vault = SecurityVault::new();

    // Interactive Menu Loop
    loop {
        print_banner();
        println!("  [1] 启动插拔卡自动锁屏/解锁守护 (前台控制台模式)");
        println!("  [2] 一键安装并启动系统后台服务 (开机自启/SYSTEM权限，推荐)");
        println!("  [3] 配置/修改 Windows Hello PIN 码");
        println!("  [4] 检查硬件、卡片与服务运行状态");
        println!("  [5] 停止并卸载系统后台服务");
        println!("  [6] 清除已绑定的 PIN 码");
        println!("  [7] 注册/修复 Windows 原生凭据提供程序 (Credential Provider)");
        println!("  [8] 退出程序");
        print!("\n请选择操作 [1-8]: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            break;
        }

        match choice.trim() {
            "1" => run_daemon(&engine, &vault),
            "2" => {
                match service::install_service() {
                    Ok(_) => println!("\n[OK] 服务已成功安装并启动！开机即可在锁屏界面自动解锁！\n"),
                    Err(e) => println!("\n[错误] {}\n", e),
                }
            }
            "3" => set_pin(&vault),
            "4" => check_status(&engine, &vault),
            "5" => {
                match service::uninstall_service() {
                    Ok(_) => println!("\n[OK] 服务已成功卸载！\n"),
                    Err(e) => println!("\n[错误] {}\n", e),
                }
            }
            "6" => clear_pin(&vault),
            "7" => {
                if service::register_credential_provider_dll() {
                    println!("\n[OK] Windows 原生凭据提供程序注册成功！\n");
                } else {
                    println!("\n[错误] 凭据提供程序注册失败，请尝试以管理员身份运行！\n");
                }
            }
            "8" | "q" | "exit" => {
                println!("程序退出。");
                break;
            }
            _ => println!("无效选项，请重新选择。\n"),
        }
    }
}
