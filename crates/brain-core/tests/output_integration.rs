use brain_core::traits::OutputSink;
use brain_common::output::{Output, OutputPayload, ResourceLifecycle};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct MockOutputSink {
    pub sent_outputs: Arc<Mutex<Vec<Output>>>,
}

#[async_trait]
impl OutputSink for MockOutputSink {
    async fn send(&self, output: Output) -> brain_common::Result<()> {
        let mut outputs = self.sent_outputs.lock().await;
        // In a real system, here we would handle the lifecycle (e.g., delete Temp resources).
        // For the mock, we just record it.
        outputs.push(output);
        Ok(())
    }
}

#[tokio::test]
async fn test_output_integration_lifecycle() {
    let mock_sink = MockOutputSink::default();
    
    // Simulate pipeline sending outputs
    let text_output = Output::text("Job completed");
    mock_sink.send(text_output).await.unwrap();

    let temp_res = Output::temp_resource("/tmp/render.md");
    mock_sink.send(temp_res).await.unwrap();

    let persistent_res = Output::persistent_resource("obsidian://vault/Entity.md");
    mock_sink.send(persistent_res).await.unwrap();

    let outputs = mock_sink.sent_outputs.lock().await;
    assert_eq!(outputs.len(), 3);
    
    assert_eq!(outputs[0].lifecycle, ResourceLifecycle::Temporary);
    assert_eq!(outputs[1].lifecycle, ResourceLifecycle::Temporary);
    assert_eq!(outputs[2].lifecycle, ResourceLifecycle::Persistent);

    match &outputs[0].payload {
        OutputPayload::InlineText { text } => assert_eq!(text, "Job completed"),
        _ => panic!("Expected InlineText"),
    }

    match &outputs[1].payload {
        OutputPayload::Resource { resource_id } => assert_eq!(resource_id, "/tmp/render.md"),
        _ => panic!("Expected Resource"),
    }
}
