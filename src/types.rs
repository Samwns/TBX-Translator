// TBX Translator - types.rs
// Creator: samwns

#[derive(Debug, Clone)]
pub enum UiMsg {
    Log(String),
    Progress(usize, usize),
    Done(String),
}
