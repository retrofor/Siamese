# Ruler - 轻量高效的 Rust 规则引擎

![License](https://img.shields.io/badge/license-MIT%202.0-green)
![Status](https://img.shields.io/badge/status-alpha-orange)

Ruler 是一个用 Rust 构建的高性能规则引擎，专为需要灵活业务逻辑的应用程序设计。它轻量、快速且易于集成，是初创项目和微服务的理想选择。

```rust
engine.add_rule(
    RuleBuilder::new("discount", "VIP User Discound")
        .condition(and![
            eq!("user_type", "VIP"),
            gt!("cart_total", 10000)
        ])
        .action(update_field!("discount", 0.15))
        .build()
);
```
- ⚡ **超高性能**：Rust 原生实现，每秒处理数十万次规则评估
- 📦 **轻量级**：零运行时依赖，编译后仅 ~300KB
- 🧩 **简单 API**：直观的构建器模式定义规则
- 🔄 **动态更新**：运行时添加/修改规则，无需重启
- 📊 **丰富类型**：支持字符串、数字、布尔值等数据类型
- 🚫 **无 unsafe 代码**：完全内存安全

## 快速开始

### 安装

添加依赖到 `Cargo.toml`：

```toml
[dependencies]
siamese = "0.1"
```

### 基本用法

```rust
use siamese::{RuleExecutor, RuleBuilder, Condition, Value};
use std::collections::HashMap;

fn main() {
    // 创建规则引擎
    let mut engine = RuleExecutor::new();
    
    // 定义折扣规则
    let discount_rule = RuleBuilder::new("discount_rule", "VIP User Discound")
        .condition(Condition::And(vec![
            Condition::Equals { field: "user_type".into(), value: Value::String("VIP".into()) },
            Condition::GreaterThan { field: "cart_total".into(), value: Value::Int(10000) }
        ]))
        .action(Action::UpdateField { 
            field: "discount".into(), 
            value: Value::Float(0.15) 
        })
        .build();
    
    engine.add_rule(discount_rule);
    
    // 准备输入数据
    let mut facts = HashMap::new();
    facts.insert("user_type".into(), Value::String("VIP".into()));
    facts.insert("cart_total".into(), Value::Int(15000));
    
    // 执行规则
    match engine.execute(&facts) {
        Ok(outputs) => {
            println!("应用折扣: {:.0}%", outputs["discount"].as_float().unwrap() * 100.0);
        }
        Err(e) => eprintln!("执行错误: {}", e),
    }
}
```

## 核心特性

### 1. 灵活的条件系统

```rust
// 复杂条件示例
Condition::And(vec![
    Condition::GreaterThan { field: "amount".into(), value: 5000.into() },
    Condition::Or(vec![
        Condition::Equals { field: "user_level".into(), value: "VIP".into() },
        Condition::Not(Box::new(
            Condition::Equals { field: "country".into(), value: "restricted".into() }
        ))
    ])
])
```

支持的条件类型：
- `Equals` (等于)
- `GreaterThan` (大于)
- `LessThan` (小于)
- `Contains` (包含)
- `And` (与)
- `Or` (或)
- `Not` (非)

### 2. 多种动作支持

| 动作类型 | 描述 | 示例 |
|----------|------|------|
| `Log` | 记录信息 | `Action::Log { message: "规则触发" }` |
| `UpdateField` | 更新字段 | `Action::UpdateField { field: "status", value: "approved" }` |
| `CallExternalService` | 调用外部服务 | `Action::CallExternalService { endpoint: "/api/verify", ... }` |
| `Composite` | 组合多个动作 | `Action::Composite(vec![action1, action2])` |

### 3. 规则优先级控制

```rust
RuleBuilder::new("high_priority", "重要规则")
    .priority(200) // 0-255，值越大优先级越高
    // ...
```

## 使用示例

### 电商促销场景

```rust
// 黑五促销规则
let black_friday_rule = RuleBuilder::new("black_friday", "黑五促销")
    .condition(Condition::Equals { 
        field: "campaign".into(), 
        value: Value::String("black_friday".into()) 
    })
    .action(Action::UpdateField { 
        field: "discount".into(), 
        value: Value::Float(0.3) 
    })
    .build();

engine.add_rule(black_friday_rule);
```

### 用户权限检查

```rust
// 管理员权限规则
let admin_rule = RuleBuilder::new("admin_access", "管理员权限")
    .condition(Condition::And(vec![
        Condition::Equals { 
            field: "role".into(), 
            value: Value::String("admin".into()) 
        },
        Condition::Equals { 
            field: "mfa_enabled".into(), 
            value: Value::Bool(true) 
        }
    ]))
    .action(Action::UpdateField { 
        field: "access_level".into(), 
        value: Value::String("full".into()) 
    })
    .build();
```

## 路线图

### v0.2 (2025 Q3)
- [ ] JSON 规则导入/导出
- [ ] 性能优化基准测试
- [ ] 更丰富的条件表达式
- [ ] 基础文档网站

### v0.3 (2025 Q4)
- [ ] WASM 支持
- [ ] SQLite 规则存储
- [ ] 规则调试工具
- [ ] Prometheus 监控集成

## 许可证

Siamese 使用 MIT 许可证分发。详情请见 [LICENSE](LICENSE) 文件。
