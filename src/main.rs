use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// 规则引擎错误类型
#[derive(Error, Debug)]
pub enum RuleEngineError {
    #[error("规则解析错误: {0}")]
    ParseError(String),
    
    #[error("规则执行错误: {0}")]
    ExecutionError(String),
    
    #[error("条件评估错误: {0}")]
    EvaluationError(String),
    
    #[error("无效规则格式: {0}")]
    InvalidRuleFormat(String),
    
    #[error("数据类型不匹配: {0}")]
    TypeMismatch(String),
    
    #[error("动作执行失败: {0}")]
    ActionFailed(String),
}

/// 规则条件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    Equals { field: String, value: Value },
    GreaterThan { field: String, value: Value },
    LessThan { field: String, value: Value },
    Contains { field: String, value: Value },
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
}

/// 规则动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Log { message: String },
    UpdateField { field: String, value: Value },
    CallExternalService { endpoint: String, payload: HashMap<String, Value> },
    SendEvent { event_type: String, data: HashMap<String, Value> },
    Composite(Vec<Action>),
}

/// 支持的数据类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Null,
}

/// 规则定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub priority: u32,
    pub condition: Condition,
    pub actions: Vec<Action>,
    pub enabled: bool,
}

/// 规则上下文
#[derive(Debug, Default)]
pub struct RuleContext {
    pub facts: HashMap<String, Value>,
    pub outputs: HashMap<String, Value>,
    pub external_data: HashMap<String, Box<dyn Any + Send + Sync>>,
}

/// 规则执行器
pub struct RuleExecutor {
    rules: Vec<Rule>,
    rule_cache: HashMap<String, Rule>,
    context: Arc<Mutex<RuleContext>>,
}

impl RuleExecutor {
    /// 创建新的规则执行器
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rule_cache: HashMap::new(),
            context: Arc::new(Mutex::new(RuleContext::default())),
        }
    }
    
    /// 添加规则
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule.clone());
        self.rule_cache.insert(rule.id.clone(), rule);
    }
    
    /// 批量添加规则
    pub fn add_rules(&mut self, rules: Vec<Rule>) {
        for rule in rules {
            self.add_rule(rule);
        }
    }
    
    /// 移除规则
    pub fn remove_rule(&mut self, rule_id: &str) {
        self.rules.retain(|r| r.id != rule_id);
        self.rule_cache.remove(rule_id);
    }
    
    /// 更新规则
    pub fn update_rule(&mut self, rule: Rule) {
        self.remove_rule(&rule.id);
        self.add_rule(rule);
    }
    
    /// 评估条件
    fn evaluate_condition(
        &self, 
        condition: &Condition, 
        facts: &HashMap<String, Value>
    ) -> Result<bool, RuleEngineError> {
        match condition {
            Condition::Equals { field, value } => {
                let fact_value = facts.get(field)
                    .ok_or_else(|| RuleEngineError::EvaluationError(
                        format!("字段不存在: {}", field)
                    ))?;
                
                if fact_value == value {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            
            Condition::GreaterThan { field, value } => {
                let fact_value = facts.get(field)
                    .ok_or_else(|| RuleEngineError::EvaluationError(
                        format!("字段不存在: {}", field)
                    ))?;
                
                match (fact_value, value) {
                    (Value::Int(a), Value::Int(b)) => Ok(a > b),
                    (Value::Float(a), Value::Float(b)) => Ok(a > b),
                    (Value::Float(a), Value::Int(b)) => Ok(*a > *b as f64),
                    (Value::Int(a), Value::Float(b)) => Ok(*a as f64 > *b),
                    _ => Err(RuleEngineError::TypeMismatch(
                        format!("无法比较 {:?} 和 {:?}", fact_value, value)
                    )),
                }
            }
            
            Condition::LessThan { field, value } => {
                let fact_value = facts.get(field)
                    .ok_or_else(|| RuleEngineError::EvaluationError(
                        format!("字段不存在: {}", field)
                    ))?;
                
                match (fact_value, value) {
                    (Value::Int(a), Value::Int(b)) => Ok(a < b),
                    (Value::Float(a), Value::Float(b)) => Ok(a < b),
                    (Value::Float(a), Value::Int(b)) => Ok(*a < *b as f64),
                    (Value::Int(a), Value::Float(b)) => Ok((*a as f64) < *b),
                    _ => Err(RuleEngineError::TypeMismatch(
                        format!("无法比较 {:?} 和 {:?}", fact_value, value)
                    )),
                }
            }
            
            Condition::Contains { field, value } => {
                let fact_value = facts.get(field)
                    .ok_or_else(|| RuleEngineError::EvaluationError(
                        format!("字段不存在: {}", field)
                    ))?;
                
                match fact_value {
                    Value::String(s) => {
                        if let Value::String(sub) = value {
                            Ok(s.contains(sub))
                        } else {
                            Err(RuleEngineError::TypeMismatch(
                                "Contains操作需要字符串".to_string()
                            ))
                        }
                    }
                    Value::List(list) => Ok(list.contains(value)),
                    _ => Err(RuleEngineError::TypeMismatch(
                        format!("字段 {} 不支持contains操作", field)
                    )),
                }
            }
            
            Condition::And(conditions) => {
                for cond in conditions {
                    if !self.evaluate_condition(cond, facts)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            
            Condition::Or(conditions) => {
                for cond in conditions {
                    if self.evaluate_condition(cond, facts)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            
            Condition::Not(condition) => {
                let result = self.evaluate_condition(condition, facts)?;
                Ok(!result)
            }
        }
    }
    
    /// 执行动作
    fn execute_action(
        &self,
        action: &Action,
        context: &mut RuleContext
    ) -> Result<(), RuleEngineError> {
        match action {
            Action::Log { message } => {
                println!("[规则引擎日志] {}", message);
                Ok(())
            }
            
            Action::UpdateField { field, value } => {
                context.outputs.insert(field.clone(), value.clone());
                Ok(())
            }
            
            Action::CallExternalService { endpoint, payload } => {
                // 实际应用中这里会调用外部服务
                println!("调用外部服务: {}, 参数: {:?}", endpoint, payload);
                // 模拟成功响应
                context.outputs.insert(
                    format!("{}_response", endpoint.replace("/", "_")),
                    Value::String("SUCCESS".to_string())
                );
                Ok(())
            }
            
            Action::SendEvent { event_type, data } => {
                // 实际应用中这里会发送事件
                println!("发送事件: {}, 数据: {:?}", event_type, data);
                Ok(())
            }
            
            Action::Composite(actions) => {
                for action in actions {
                    self.execute_action(action, context)?;
                }
                Ok(())
            }
        }
    }
    
    /// 执行规则
    pub fn execute(&self, facts: &HashMap<String, Value>) -> Result<HashMap<String, Value>, RuleEngineError> {
        let mut context = RuleContext {
            facts: facts.clone(),
            outputs: HashMap::new(),
            external_data: HashMap::new(),
        };
        
        // 按优先级排序规则（优先级数值高的先执行）
        let mut sorted_rules = self.rules.clone();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        for rule in sorted_rules {
            if !rule.enabled {
                continue;
            }
            
            if self.evaluate_condition(&rule.condition, facts)? {
                println!("规则触发: {} ({})", rule.name, rule.id);
                for action in &rule.actions {
                    self.execute_action(action, &mut context)?;
                }
            }
        }
        
        Ok(context.outputs)
    }
}

/// 规则构建器
pub struct RuleBuilder {
    rule: Rule,
}

impl RuleBuilder {
    pub fn new(id: &str, name: &str) -> Self {
        RuleBuilder {
            rule: Rule {
                id: id.to_string(),
                name: name.to_string(),
                description: None,
                priority: 50,
                condition: Condition::And(vec![]),
                actions: vec![],
                enabled: true,
            },
        }
    }
    
    pub fn description(mut self, description: &str) -> Self {
        self.rule.description = Some(description.to_string());
        self
    }
    
    pub fn priority(mut self, priority: u32) -> Self {
        self.rule.priority = priority;
        self
    }
    
    pub fn condition(mut self, condition: Condition) -> Self {
        self.rule.condition = condition;
        self
    }
    
    pub fn action(mut self, action: Action) -> Self {
        self.rule.actions.push(action);
        self
    }
    
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.rule.enabled = enabled;
        self
    }
    
    pub fn build(self) -> Rule {
        self.rule
    }
}

fn main() {
    // 创建规则执行器
    let mut engine = RuleExecutor::new();
    
    // 创建规则1: 高风险交易检测
    let rule1 = RuleBuilder::new("rule1", "高风险交易检测")
        .description("检测高风险交易")
        .priority(100)
        .condition(Condition::And(vec![
            Condition::GreaterThan {
                field: "amount".to_string(),
                value: Value::Int(10000),
            },
            Condition::Equals {
                field: "currency".to_string(),
                value: Value::String("USD".to_string()),
            },
            Condition::Or(vec![
                Condition::Equals {
                    field: "country".to_string(),
                    value: Value::String("高风险国家1".to_string()),
                },
                Condition::Equals {
                    field: "country".to_string(),
                    value: Value::String("高风险国家2".to_string()),
                },
            ]),
        ]))
        .action(Action::Log {
            message: "检测到高风险交易".to_string(),
        })
        .action(Action::CallExternalService {
            endpoint: "/fraud-detection".to_string(),
            payload: HashMap::from([
                ("transaction_id".to_string(), Value::String("txn12345".to_string())),
                ("amount".to_string(), Value::Int(15000)),
            ]),
        })
        .build();
    
    // 创建规则2: 促销活动
    let rule2 = RuleBuilder::new("rule2", "大额折扣促销")
        .priority(80)
        .condition(Condition::And(vec![
            Condition::GreaterThan {
                field: "amount".to_string(),
                value: Value::Int(5000),
            },
            Condition::Not(Box::new(Condition::Equals {
                field: "category".to_string(),
                value: Value::String("电子产品".to_string()),
            })),
        ]))
        .action(Action::UpdateField {
            field: "discount".to_string(),
            value: Value::Float(0.15),
        })
        .action(Action::SendEvent {
            event_type: "promotion_applied".to_string(),
            data: HashMap::from([
                ("discount".to_string(), Value::Float(0.15)),
                ("rule".to_string(), Value::String("大额折扣".to_string())),
            ]),
        })
        .build();
    
    // 添加规则到引擎
    engine.add_rule(rule1);
    engine.add_rule(rule2);
    
    // 准备事实数据
    let facts = HashMap::from([
        ("amount".to_string(), Value::Int(15000)),
        ("currency".to_string(), Value::String("USD".to_string())),
        ("country".to_string(), Value::String("高风险国家1".to_string())),
        ("category".to_string(), Value::String("服装".to_string())),
    ]);
    
    // 执行规则
    match engine.execute(&facts) {
        Ok(outputs) => {
            println!("规则执行结果: {:?}", outputs);
        }
        Err(e) => {
            eprintln!("规则执行错误: {}", e);
        }
    }
}