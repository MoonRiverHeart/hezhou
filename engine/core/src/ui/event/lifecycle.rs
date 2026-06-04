use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleEvent {
    pub timestamp: SystemTime,
    pub event_type: LifecycleEventType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleEventType {
    /// 应用启动
    AppStarted,
    /// 应用即将退出
    AppWillExit,
    /// 应用已退出
    AppExited,
    /// 进入后台
    EnteredBackground,
    /// 进入前台
    EnteredForeground,
    /// 内存警告
    MemoryWarning,
}