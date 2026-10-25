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
    println!("  Windows 智能卡解锁系统 (RTS5169 + IC4442) [Rust 原生极速引擎] ");
    println!("==============================================================");
}

fn print_help() {
    print_banner();
    println!("用法:");
    println!("  smartcard_unlock.exe [命令] [参数]\n");
    println!("命令列表:");
    println!("  set-pin [密码]       配置或更新 Windows 解锁密码 (不带参数则进入交互输入)");
    println!("  status               查看当前硬件、卡片、服务与凭据提供程序状态");
    println!("  install-service      一键安装并启动后台服务 (需要管理员权限)");
    println!("  uninstall-service    停止并卸载后台服务 (需要管理员权限)");
    println!("  start-service        启动已安装的后台服务 (需要管理员权限)");
    println!("  stop-service         停止后台服务 (需要管理员权限)");
    println!("  register-credprov    注册系统级 Windows 原生凭据提供程序 (需要管理员权限)");
    println!("  run | daemon         前台运行实时插拔卡守护模式");
    println!("  lock-now             立即触发原生工作站锁屏");
    println!("  clear                清除已保存的凭据数据");
    println!("  help | --help | -h   显示本帮助信息\n");
    println!("说明:");
    println!("  直接运行程序 (不带任何参数) 将进入交互式控制台菜单。");
}

fn run_daemon(engine: &mut SmartCardEngine, vault: &SecurityVault) {
    lock::enable_privilege("SeDebugPrivilege");
    lock::enable_privilege("SeImpersonatePrivilege");

    if !vault.is_enrolled() {
        println!("\n[提示]当前尚未配置解锁密码");
        print!("请输入您电脑的Windows账户本地密码绑定: ");
        io::stdout().flush().unwrap();
        let mut pin = String::new();
        io::stdin().read_line(&mut pin).unwrap();
        let pin = pin.trim();
        if pin.is_empty() {
            println!("[错误] 密码不能为空");
            return;
        }
        if vault.save_pin(pin) {
            println!("[OK] 解锁密码已存储");
        } else {
            println!("[错误] 凭据加密失败！");
            return;
        }
    }

    if let None = vault.load_pin() {
        println!("[错误] 无法解密密码，请重新配置！");
        return;
    }

    println!("\n[OK] 智能卡守护已启动");

    let mut last_present: Option<bool> = None;
    let mut last_reader = String::new();

    loop {
        let readers = engine.list_readers();
        if readers.is_empty() {
            if !last_reader.is_empty() {
                println!("\n  [硬件断开] 检测到读卡器 USB 已断开连接！进入重连状态...");
                last_reader.clear();
                if let Some(true) = last_present {
                    last_present = Some(false);
                    let session_id = lock::get_active_session_id();
                    println!("读卡器拔出，执行锁屏 (会话 ID: {})...", session_id);
                    lock::lock_session(session_id);
                }
            }
            thread::sleep(Duration::from_millis(500));
            continue;
        }

        let target_reader = readers
            .iter()
            .find(|r| {
                let lower = r.to_lowercase();
                lower.contains("emv") || lower.contains("rts") || lower.contains("realtek")
            })
            .unwrap_or(&readers[0]);

        if target_reader != &last_reader {
            last_reader = target_reader.clone();
            println!("\n [硬件重连] 成功识别读卡器: {}", last_reader);
        }

        let (present, _event_state) = engine.get_card_status(target_reader);

        match last_present {
            None => {
                last_present = Some(present);
                if present {
                    println!("[状态] 当前读卡器中已检测到卡片。");
                } else {
                    println!("[状态] 当前读卡器为空，等待插卡...");
                }
            }
            Some(prev) => {
                if prev != present {
                    last_present = Some(present);
                    let session_id = lock::get_active_session_id();

                    if present {
                        println!("\n [事件] 检测到卡片插入！(会话 ID: {})", session_id);
                        if lock::is_session_locked(session_id) {
                            println!("   -> 当前系统处于锁定状态，已传递解锁");
                        } else {
                            println!("   -> 当前系统未锁定");
                        }
                    } else {
                        println!("\n [事件] 检测到卡片已拔出");
                        println!("   -> 立即执行锁屏...");
                        let res = lock::lock_session(session_id);
                        if res {
                            println!("   -> [成功] 系统已锁定");
                        } else {
                            println!("   -> [提示] 锁屏调用已触发");
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn check_status(engine: &mut SmartCardEngine, vault: &SecurityVault) {
    println!("\n--- 硬件与安全状态诊断 ---");
    let readers = engine.list_readers();
    println!("1. 发现的读卡器 ({} 个):", readers.len());
    for r in &readers {
        println!("   -> {}", r);
    }

    if let Some(target) = readers.first() {
        let (present, state) = engine.get_card_status(target);
        println!("2. 卡片物理状态: {}", if present { "已插入" } else { "未插入" });
        println!("   状态标志字: 0x{:08X}", state);
    }

    println!("3. Windows 密码绑定状态: {}", if vault.is_enrolled() { "已绑定" } else { "未绑定" });

    let session_id = lock::get_active_session_id();
    let locked = lock::is_session_locked(session_id);
    println!("4. Windows 会话锁定状态: {} (会话 ID: {})", if locked { "已锁定" } else { "活动中" }, session_id);

    let (installed, running) = service::is_service_installed_and_running();
    println!("5. Windows 后台系统服务状态: {}", 
        if !installed {
            "未安装"
        } else if running {
            "已安装并正在运行"
        } else {
            "已安装但已停止"
        }
    );

    let credprov_registered = service::is_credential_provider_registered();
    println!("6. Windows 原生凭据提供程序状态: {}",
        if credprov_registered {
            "已注册到系统 (CLSID: {7C3B984A-94B6-4E87-8A33-8FE58D6E9A42})"
        } else {
            "未注册 (可通过菜单【7】进行注册)"
        }
    );
    println!("--------------------------\n");
}

fn set_pin(vault: &SecurityVault) {
    print!("\n请输入要绑定的Windows账户本地密码: ");
    io::stdout().flush().unwrap();
    let mut pin = String::new();
    io::stdin().read_line(&mut pin).unwrap();
    let pin = pin.trim();

    if pin.is_empty() {
        println!("[错误] 密码不能为空！");
        return;
    }

    if vault.save_pin(pin) {
        println!("[OK] 密码保存成功\n");
    } else {
        println!("[错误] 保存密码失败\n");
    }
}

fn clear_pin(vault: &SecurityVault) {
    if vault.clear() {
        println!("\n[OK] 已清除绑定的密码\n");
    } else {
        println!("\n[错误] 清除失败\n");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "help" | "--help" | "-h" | "/?" => {
                print_help();
                return;
            }
            "service-run" => {
                service::run_service();
                return;
            }
            "lock-now" => {
                let session_id = lock::get_active_session_id();
                lock::lock_session(session_id);
                return;
            }
            "install-service" => {
                match service::install_service() {
                    Ok(_) => println!("[OK] 服务安装成功并已启动"),
                    Err(e) => eprintln!("[错误] 安装服务失败: {}", e),
                }
                return;
            }
            "uninstall-service" => {
                match service::uninstall_service() {
                    Ok(_) => println!("[OK] 服务已卸载"),
                    Err(e) => eprintln!("[错误] 卸载服务失败: {}", e),
                }
                return;
            }
            "start-service" => {
                match service::start_service() {
                    Ok(_) => println!("[OK] 服务已启动"),
                    Err(e) => eprintln!("[错误] 启动服务失败: {}", e),
                }
                return;
            }
            "stop-service" => {
                match service::stop_service() {
                    Ok(_) => println!("[OK] 服务已停止"),
                    Err(e) => eprintln!("[错误] 停止服务失败: {}", e),
                }
                return;
            }
            "run" | "daemon" => {
                let mut engine = match SmartCardEngine::new() {
                    Ok(e) => e,
                    Err(err) => {
                        eprintln!("[严重错误] 无法初始化 Windows PC/SC 智能卡服务 (代码: 0x{:08X})", err);
                        return;
                    }
                };
                let vault = SecurityVault::new();
                run_daemon(&mut engine, &vault);
                return;
            }
            "status" => {
                let mut engine = match SmartCardEngine::new() {
                    Ok(e) => e,
                    Err(err) => {
                        eprintln!("[严重错误] 无法初始化 Windows PC/SC 智能卡服务 (代码: 0x{:08X})", err);
                        return;
                    }
                };
                let vault = SecurityVault::new();
                check_status(&mut engine, &vault);
                return;
            }
            "set-pin" => {
                let vault = SecurityVault::new();
                if args.len() > 2 {
                    if vault.save_pin(args[2].trim()) {
                        println!("[OK] 密码设置成功");
                    } else {
                        println!("[错误] 密码保存失败");
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
                    println!("[OK] Windows 凭据提供程序注册成功！");
                } else {
                    println!("[错误] 凭据提供程序注册失败，请确保以管理员权限或系统服务运行");
                }
                return;
            }
            _ => {}
        }
    }

    let mut engine = match SmartCardEngine::new() {
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
        println!("  [1] 启动前台控制台模式");
        println!("  [2] 一键安装并启动系统后台服务");
        println!("  [3] 配置/修改 Windows 解锁密码");
        println!("  [4] 检查硬件、卡片与服务运行状态");
        println!("  [5] 停止并卸载系统后台服务");
        println!("  [6] 清除已绑定的密码");
        println!("  [7] 注册/修复 Windows 凭据提供程序");
        println!("  [8] 退出程序");
        print!("\n请选择操作 [1-8]: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            break;
        }

        match choice.trim() {
            "1" => run_daemon(&mut engine, &vault),
            "2" => {
                match service::install_service() {
                    Ok(_) => println!("\n[OK] 服务已成功安装并启动\n"),
                    Err(e) => println!("\n[错误] {}\n", e),
                }
            }
            "3" => set_pin(&vault),
            "4" => check_status(&mut engine, &vault),
            "5" => {
                match service::uninstall_service() {
                    Ok(_) => println!("\n[OK] 服务已成功卸载\n"),
                    Err(e) => println!("\n[错误] {}\n", e),
                }
            }
            "6" => clear_pin(&vault),
            "7" => {
                if service::register_credential_provider_dll() {
                    println!("\n[OK] Windows 凭据提供程序注册成功\n");
                } else {
                    println!("\n[错误] 凭据提供程序注册失败，请尝试以管理员身份运行\n");
                }
            }
            "8" | "q" | "exit" => {
                println!("程序退出");
                break;
            }
            _ => println!("无效选项，请重新选择\n"),
        }
    }
}
