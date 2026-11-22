use keynear_core::{AudioHandoffEngine, AudioHandoffPlan, AudioSinkProfile, KeynearError};

use super::dbus;

pub trait HandoffControl {
    async fn pause_players(&self) -> Result<usize, KeynearError>;
    async fn resume_players(&self) -> Result<usize, KeynearError>;
    async fn release_sink(&self, profile: AudioSinkProfile) -> Result<(), KeynearError>;
    async fn claim_sink(&self, profile: AudioSinkProfile) -> Result<(), KeynearError>;
}

pub struct DbusHandoffControl;

impl HandoffControl for DbusHandoffControl {
    async fn pause_players(&self) -> Result<usize, KeynearError> {
        dbus::call_players("Pause").await
    }

    async fn resume_players(&self) -> Result<usize, KeynearError> {
        dbus::call_players("Play").await
    }

    async fn release_sink(&self, profile: AudioSinkProfile) -> Result<(), KeynearError> {
        tracing::info!(event = "vega_sink_release", profile = profile.as_str());
        Ok(())
    }

    async fn claim_sink(&self, profile: AudioSinkProfile) -> Result<(), KeynearError> {
        tracing::info!(event = "vega_sink_claim", profile = profile.as_str());
        Ok(())
    }
}

pub struct VegaAudioController<C: HandoffControl> {
    engine: AudioHandoffEngine,
    control: C,
}

impl<C: HandoffControl> VegaAudioController<C> {
    pub fn new(control: C) -> Self {
        Self {
            engine: AudioHandoffEngine::new(),
            control,
        }
    }

    pub fn snapshot(&self) -> Option<&keynear_core::AudioHandoffState> {
        self.engine.snapshot()
    }

    pub async fn begin(
        &mut self,
        sequence: u64,
        peripheral_id: &str,
        profile: AudioSinkProfile,
    ) -> Result<(), KeynearError> {
        let plan = self.engine.begin(sequence, peripheral_id, profile)?;
        self.apply(&plan, profile).await
    }

    pub async fn mark_released(&mut self, sequence: u64) -> Result<(), KeynearError> {
        self.engine.mark_released(sequence)
    }

    pub async fn peer_claimed(&mut self, sequence: u64) -> Result<(), KeynearError> {
        self.engine.peer_claimed(sequence)
    }

    pub async fn finish(&mut self, sequence: u64) -> Result<(), KeynearError> {
        let profile = self.engine.snapshot().map(|state| state.profile);
        let plan = self.engine.release(sequence)?;
        if let Some(profile) = profile {
            self.apply(&plan, profile).await?;
        }
        self.engine.clear();
        Ok(())
    }

    pub async fn abort(&mut self, sequence: u64) -> Result<(), KeynearError> {
        let profile = self.engine.snapshot().map(|state| state.profile);
        if let Some(plan) = self.engine.abort(sequence) {
            if let Some(profile) = profile {
                self.apply(&plan, profile).await?;
            }
        }
        self.engine.clear();
        Ok(())
    }

    async fn apply(
        &self,
        plan: &AudioHandoffPlan,
        profile: AudioSinkProfile,
    ) -> Result<(), KeynearError> {
        if plan.pause_playback {
            self.control.pause_players().await?;
        }
        if plan.release_sink {
            self.control.release_sink(profile).await?;
        }
        if plan.reclaim_sink {
            self.control.claim_sink(profile).await?;
        }
        if plan.resume_playback {
            self.control.resume_players().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Recorder {
        calls: Mutex<Vec<&'static str>>,
    }

    impl Recorder {
        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().expect("recorder").clone()
        }
    }

    struct RecordingControl(Arc<Recorder>);

    impl HandoffControl for RecordingControl {
        async fn pause_players(&self) -> Result<usize, KeynearError> {
            self.0.calls.lock().expect("recorder").push("pause");
            Ok(1)
        }

        async fn resume_players(&self) -> Result<usize, KeynearError> {
            self.0.calls.lock().expect("recorder").push("resume");
            Ok(1)
        }

        async fn release_sink(&self, _profile: AudioSinkProfile) -> Result<(), KeynearError> {
            self.0.calls.lock().expect("recorder").push("release");
            Ok(())
        }

        async fn claim_sink(&self, _profile: AudioSinkProfile) -> Result<(), KeynearError> {
            self.0.calls.lock().expect("recorder").push("claim");
            Ok(())
        }
    }

    fn controller() -> (VegaAudioController<RecordingControl>, Arc<Recorder>) {
        let recorder = Arc::new(Recorder::default());
        let control = RecordingControl(Arc::clone(&recorder));
        (VegaAudioController::new(control), recorder)
    }

    #[tokio::test]
    async fn handoff_pauses_before_release_and_claims_before_resume() {
        let (mut controller, recorder) = controller();
        controller
            .begin(1, "AA:BB:CC:DD:EE:FF", AudioSinkProfile::BapSink)
            .await
            .expect("begin");
        controller.mark_released(2).await.expect("released");
        controller.peer_claimed(3).await.expect("claimed");
        controller.finish(4).await.expect("finish");
        assert_eq!(recorder.calls(), ["pause", "release", "claim", "resume"]);
    }

    #[tokio::test]
    async fn aborted_handoff_reclaims_and_resumes() {
        let (mut controller, recorder) = controller();
        controller
            .begin(1, "headset", AudioSinkProfile::HfpHf)
            .await
            .expect("begin");
        controller.mark_released(2).await.expect("released");
        controller.abort(3).await.expect("abort");
        assert_eq!(recorder.calls(), ["pause", "release", "claim", "resume"]);
    }
}
