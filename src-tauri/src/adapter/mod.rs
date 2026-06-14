/// 代理适配器模块。
///
/// 提供统一的 Agent 适配器接口，支持多代理（Pi、Hermes、OpenClaw）的事件监听和消息发送。
///
/// # 架构
///
/// 适配器模块遵循开闭原则：新增 Agent 只需实现 [`AgentAdapter`] trait，无需修改核心代码。
///
/// ## 模块组成
///
/// - `trait` — 定义 [`AgentAdapter`] trait，所有适配器必须实现此接口
/// - `pi_adapter` — Pi Agent 适配器核心实现，通过 JSONL 文件监听事件
/// - `pi_watcher` — Pi JSONL 文件监听器，跨平台监控日志文件新行
/// - `event_converter` — Pi 事件转换器，将 Pi 原生 JSON 事件转换为 [`UnifiedAgentEvent`]
///
/// # 使用示例
///
/// ```no_run
/// use agent_pet_hub_lib::adapter::{AgentAdapter, PiAdapter, PiAdapterConfig};
/// use agent_pet_hub_lib::event_bus::EventBus;
/// use agent_pet_hub_lib::state_machine::PetStateMachine;
/// use agent_pet_hub_lib::types::{UnifiedAgentEvent, PetState};
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// # #[tokio::main]
/// # async fn main() {
/// let event_bus = EventBus::new(1024);
/// let state_machine = Arc::new(Mutex::new(PetStateMachine::new()));
///
/// let adapter = PiAdapter::new(
///     PiAdapterConfig::default(),
///     event_bus,
///     state_machine,
/// );
///
/// adapter.connect().await.expect("Failed to connect");
/// # }
/// ```

mod r#trait;
#[allow(unused_imports)]
pub use r#trait::{AdapterError, AdapterIdentity, AgentAdapter};

mod pi_adapter;
pub use pi_adapter::{PiAdapter, PiAdapterConfig};

mod pi_watcher;

mod event_converter;
