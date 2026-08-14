// TBX Translator - types.rs
// Creator: samwns

#[derive(Debug, Clone)]
pub enum UiMsg {
    Log(String),
    Progress(usize, usize),
    Done(String),
    Error(String),
    Cancelled,
    DetectedLanguageMismatch(String),
    EngineDone(usize, String),
    EngineError(usize, String),
    EngineCancelled(usize),
    EngineLog(usize, String),
    EngineProgress(usize, usize, usize),
    UpdateFound(crate::updater::ReleaseInfo),
    UpdateStatus(String),
    UpdateProgress(u64, u64),
    UpdateError(String),
}
