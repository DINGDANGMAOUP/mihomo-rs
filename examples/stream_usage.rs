//! 流式接口使用示例
//!
//! 本示例演示如何使用 mihomo-rs SDK 的流式接口来持续监控流量和内存使用情况。
//! 这些接口会持续返回数据流，适合用于实时监控场景。

use futures_util::StreamExt;
use mihomo_rs::{MihomoClient, Result};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // 创建客户端
    let client = MihomoClient::new("http://127.0.0.1:9090", None)?;

    println!("=== Mihomo 流式接口使用示例 ===");
    println!();

    // 获取版本信息
    match client.version().await {
        Ok(version) => {
            println!("✅ 连接到 Mihomo 成功");
            println!("版本: {}", version.version);
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            return Err(e);
        }
    }
    println!();

    // 演示流量监控流
    println!("🚀 开始流量监控 (10秒)...");
    if let Err(e) = monitor_traffic_stream(&client).await {
        println!("⚠️  流量监控出错: {}", e);
    }
    println!();

    // 演示内存监控流
    println!("🧠 开始内存监控 (10秒)...");
    if let Err(e) = monitor_memory_stream(&client).await {
        println!("⚠️  内存监控出错: {}", e);
    }
    println!();

    // 演示同时监控流量和内存
    println!("📊 同时监控流量和内存 (15秒)...");
    if let Err(e) = monitor_both_streams(&client).await {
        println!("⚠️  同时监控出错: {}", e);
    }

    println!("✅ 流式监控示例完成");
    Ok(())
}

/// 监控流量统计流
async fn monitor_traffic_stream(client: &MihomoClient) -> Result<()> {
    let stream = client.traffic_stream().await?;
    let mut stream = stream.take(10); // 只取前10个数据点

    let mut count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(traffic) => {
                count += 1;
                println!(
                    "[{}] 流量统计 - 上传: {} bytes, 下载: {} bytes",
                    count,
                    format_bytes(traffic.up),
                    format_bytes(traffic.down)
                );
            }
            Err(e) => {
                println!("❌ 流量数据解析错误: {}", e);
            }
        }
    }

    Ok(())
}

/// 监控内存使用情况流
async fn monitor_memory_stream(client: &MihomoClient) -> Result<()> {
    let stream = client.memory_stream().await?;
    let mut stream = stream.take(10); // 只取前10个数据点

    let mut count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(memory) => {
                count += 1;
                println!(
                    "[{}] 内存使用 - 当前: {}, 系统限制: {}",
                    count,
                    format_bytes(memory.in_use),
                    if memory.os_limit > 0 {
                        format_bytes(memory.os_limit)
                    } else {
                        "无限制".to_string()
                    }
                );
            }
            Err(e) => {
                println!("❌ 内存数据解析错误: {}", e);
            }
        }
    }

    Ok(())
}

/// 同时监控流量和内存
async fn monitor_both_streams(client: &MihomoClient) -> Result<()> {
    let traffic_stream = client.traffic_stream().await?;
    let memory_stream = client.memory_stream().await?;

    // 使用 select! 宏来同时处理两个流
    let mut traffic_stream = traffic_stream.take(15);
    let mut memory_stream = memory_stream.take(15);

    let mut traffic_count = 0;
    let mut memory_count = 0;

    // 设置超时时间
    let monitor_future = async {
        loop {
            tokio::select! {
                traffic_result = traffic_stream.next() => {
                    match traffic_result {
                        Some(Ok(traffic)) => {
                            traffic_count += 1;
                            println!(
                                "📈 [流量-{}] 上传: {}, 下载: {}",
                                traffic_count,
                                format_bytes(traffic.up),
                                format_bytes(traffic.down)
                            );
                        }
                        Some(Err(e)) => {
                            println!("❌ 流量数据错误: {}", e);
                        }
                        None => {
                            println!("📈 流量监控结束");
                            break;
                        }
                    }
                }
                memory_result = memory_stream.next() => {
                    match memory_result {
                        Some(Ok(memory)) => {
                            memory_count += 1;
                            println!(
                                "🧠 [内存-{}] 使用: {}",
                                memory_count,
                                format_bytes(memory.in_use)
                            );
                        }
                        Some(Err(e)) => {
                            println!("❌ 内存数据错误: {}", e);
                        }
                        None => {
                            println!("🧠 内存监控结束");
                            break;
                        }
                    }
                }
            }
        }
    };

    // 设置15秒超时
    match timeout(Duration::from_secs(15), monitor_future).await {
        Ok(_) => println!("✅ 监控正常结束"),
        Err(_) => println!("⏰ 监控超时结束"),
    }

    Ok(())
}

/// 格式化字节数为可读格式
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}
