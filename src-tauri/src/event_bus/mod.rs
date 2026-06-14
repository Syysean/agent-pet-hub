/// 事件总线模块。
///
/// # 默认通道大小
///
/// 默认 channel_size 为 4096，确保高吞吐场景下事件不会丢失。

/// 事件总线默认通道大小。
pub const DEFAULT_CHANNEL_SIZE: usize = 4096;
///
/// 提供基于 `tokio::sync::broadcast` 的发布/订阅系统，
/// 用于在 agent-pet-hub 内部传播统一代理事件和状态变更。
///
/// # 架构
///
/// 事件总线将事件流拆分为两条通道：
///
/// - **事件通道** (`UnifiedAgentEvent`) — 所有代理事件的广播流
/// - **状态通道** (`(PetState, PetState)`) — 状态变更的广播流
///
/// # 使用示例
///
/// ```no_run
/// use agent_pet_hub_lib::event_bus::EventBus;
///
/// let bus = EventBus::new(1024);
///
/// // 创建订阅者
/// let mut event_rx = bus.subscribe_event();
/// let mut state_rx = bus.subscribe_state();
///
/// // 发送事件（在另一个任务中）
/// let bus_clone = bus.clone();
/// tokio::spawn(async move {
///     bus_clone.publish_heartbeat().ok();
/// });
/// ```

mod bus;

pub use bus::EventBus;
