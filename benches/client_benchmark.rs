//! 客户端性能基准测试

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mihomo_rs::client::MihomoClient;
use tokio::runtime::Runtime;

/// 基准测试客户端创建性能
fn bench_client_creation(c: &mut Criterion) {
    c.bench_function("client_creation", |b| {
        b.iter(|| {
            let client = MihomoClient::new(
                black_box("http://127.0.0.1:9090"),
                black_box(None)
            );
            black_box(client)
        })
    });
}

/// 基准测试配置获取性能
fn bench_config_fetch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("config_fetch", |b| {
        b.to_async(&rt).iter(|| async {
            let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
            let result = client.get_config().await;
            black_box(result)
        })
    });
}

/// 基准测试代理列表获取性能
fn bench_proxies_fetch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("proxies_fetch", |b| {
        b.to_async(&rt).iter(|| async {
            let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
            let result = client.get_proxies().await;
            black_box(result)
        })
    });
}

/// 基准测试规则获取性能
fn bench_rules_fetch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("rules_fetch", |b| {
        b.to_async(&rt).iter(|| async {
            let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
            let result = client.get_rules().await;
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    bench_client_creation,
    bench_config_fetch,
    bench_proxies_fetch,
    bench_rules_fetch
);
criterion_main!(benches);