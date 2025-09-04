//! 规则引擎性能基准测试

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mihomo_rs::{client::MihomoClient, rules::RuleEngine, types::{Rule, RuleType}};
use tokio::runtime::Runtime;

/// 基准测试规则匹配性能
fn bench_rule_matching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("rule_matching", |b| {
        b.to_async(&rt).iter(|| async {
            let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
            let mut rule_engine = RuleEngine::new(client);
            
            let result = rule_engine.match_rule(
                black_box("example.com"),
                black_box(Some(443)),
                black_box(None)
            ).await;
            black_box(result)
        })
    });
}

/// 基准测试大量规则匹配性能
fn bench_bulk_rule_matching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let test_domains = vec![
        "google.com",
        "github.com",
        "stackoverflow.com",
        "rust-lang.org",
        "example.com",
        "cloudflare.com",
        "microsoft.com",
        "apple.com",
        "amazon.com",
        "facebook.com",
    ];
    
    c.bench_function("bulk_rule_matching", |b| {
        b.to_async(&rt).iter(|| async {
            let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
            let mut rule_engine = RuleEngine::new(client);
            
            for domain in &test_domains {
                let result = rule_engine.match_rule(
                    black_box(domain),
                    black_box(Some(443)),
                    black_box(None)
                ).await;
                black_box(result);
            }
        })
    });
}

/// 基准测试规则创建性能
fn bench_rule_creation(c: &mut Criterion) {
    c.bench_function("rule_creation", |b| {
        b.iter(|| {
            let rule = Rule {
                rule_type: black_box(RuleType::Domain),
                payload: black_box("example.com".to_string()),
                proxy: black_box("DIRECT".to_string()),
                size: black_box(0),
            };
            black_box(rule)
        })
    });
}

/// 基准测试大量规则创建性能
fn bench_bulk_rule_creation(c: &mut Criterion) {
    c.bench_function("bulk_rule_creation", |b| {
        b.iter(|| {
            let mut rules = Vec::new();
            for i in 0..1000 {
                let rule = Rule {
                    rule_type: black_box(RuleType::Domain),
                    payload: black_box(format!("example{}.com", i)),
                    proxy: black_box("DIRECT".to_string()),
                    size: black_box(0),
                };
                rules.push(rule);
            }
            black_box(rules)
        })
    });
}

criterion_group!(
    benches,
    bench_rule_matching,
    bench_bulk_rule_matching,
    bench_rule_creation,
    bench_bulk_rule_creation
);
criterion_main!(benches);