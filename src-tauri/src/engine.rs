use crate::mode::ModeLanguage;
use crate::recorder::AudioFormat;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct EngineOutcome {
    pub transcript: String,
    pub warning: Option<Warning>,
}

pub enum Warning {
    FinalFailedUsedPreview,
}

#[allow(async_fn_in_trait)]
pub trait Engine {
    async fn run(
        self,
        chunks: UnboundedReceiver<Vec<i16>>,
        previews: UnboundedSender<String>,
        ctx: EngineContext,
    ) -> Result<EngineOutcome, String>;
}

pub struct EngineContext {
    pub format: AudioFormat,
    pub language: ModeLanguage,
    pub terms: Vec<String>,
}
