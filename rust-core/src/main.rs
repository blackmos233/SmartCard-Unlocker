mod scard;
mod lock;
mod vault;

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

    let pin = match vault.load_pin() {
        Some(p) => p,
        None => {
            println!("[错误] 无法解密 PIN 码，请重新配置！");
            return;
        }
    };

    println!("\n[OK] 智能卡安全守护已启动！(按 Ctrl+C 可停止)");
    println!("     -> 拔出 IC4442 卡片: 毫秒级触发 Windows 原生锁屏");
    println!("     -> 插入 IC4442 卡片: 自动唤醒并模拟输入 PIN 码解锁\n");

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
                // Initial detection
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
                    if present {
                        println!("\n💳 [事件] 检测到 IC4442 卡片插入！");
                        if lock::is_workstation_locked() {
                            println!("   -> 当前系统处于锁定状态，正在执行自动解锁 (PIN 注入)...");
                            lock::inject_pin_keystrokes(&pin);
                            println!("   -> 已发送 PIN 码解锁指令！");
                        } else {
                            println!("   -> 当前系统未锁定，保持正常使用。");
                        }
                    } else {
                        println!("\n🔒 [事件] 检测到 IC4442 卡片已拔出！");
                        println!("   -> 立即执行 Windows 原生锁屏 (LockWorkStation)...");
                        let res = lock::lock_workstation();
                        if res {
                            println!("   -> [成功] 系统已安全锁定！");
                        } else {
                            println!("   -> [提示] 锁屏调用已触发。");
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(250));
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
    println!("4. Windows 会话锁定状态: {}", if lock::is_workstation_locked() { "已锁定" } else { "活动中" });
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
        println!("[OK] Windows Hello PIN 码保存成功（已使用 DPAPI 双重加密）！\n");
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

    let engine = match SmartCardEngine::new() {
        Ok(e) => e,
        Err(err) => {
            eprintln!("[严重错误] 无法初始化 Windows PC/SC 智能卡服务 (代码: 0x{:08X})", err);
            return;
        }
    };
    let vault = SecurityVault::new();

    if args.len() > 1 {
        match args[1].as_str() {
            "run" | "daemon" => {
                run_daemon(&engine, &vault);
                return;
            }
            "status" => {
                check_status(&engine, &vault);
                return;
            }
            "set-pin" => {
                if args.len() > 2 {
                    if vault.save_pin(args[2].trim()) {
                        println!("[OK] PIN 码设置成功！");
                    }
                } else {
                    set_pin(&vault);
                }
                return;
            }
            "clear" => {
                clear_pin(&vault);
                return;
            }
            _ => {}
        }
    }

    // Interactive Menu Loop
    loop {
        print_banner();
        println!("  [1] 启动插拔卡自动锁屏/解锁守护");
        println!("  [2] 配置/修改 Windows Hello PIN 码");
        println!("  [3] 检查读卡器与卡片连接状态");
        println!("  [4] 清除已绑定的 PIN 码");
        println!("  [5] 退出程序");
        print!("\n请选择操作 [1-5]: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            break;
        }

        match choice.trim() {
            "1" => run_daemon(&engine, &vault),
            "2" => set_pin(&vault),
            "3" => check_status(&engine, &vault),
            "4" => clear_pin(&vault),
            "5" | "q" | "exit" => {
                println!("程序退出。");
                break;
            }
            _ => println!("无效选项，请重新选择。\n"),
        }
    }
}
