use crate::mode::ModeLanguage;
use crate::recorder::AudioFormat;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

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

pub struct EngineOutcome {
    pub transcript: String,
    pub warning: Option<Warning>,
}

pub enum Warning {
    FinalFailedUsedPreview,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_with_no_warning_has_none() {
        let outcome = EngineOutcome {
            transcript: "hello".to_string(),
            warning: None,
        };
        assert!(outcome.warning.is_none());
        assert_eq!(outcome.transcript, "hello");
    }

    #[test]
    fn outcome_with_warning_has_some() {
        let outcome = EngineOutcome {
            transcript: String::new(),
            warning: Some(Warning::FinalFailedUsedPreview),
        };
        assert!(outcome.warning.is_some());
    }
}
