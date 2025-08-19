use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

/// Channel buffer size for endpoint callbacks
const CALLBACK_CHANNEL_SIZE: usize = 50;

/// Identifier for global overrides that apply to all RPC calls
const GLOBAL_OVERRIDE_KEY: &str = "global";

use tokio::sync::mpsc;

/// Result of waiting for an endpoint to be triggered
#[derive(Debug, Clone)]
pub enum CallbackResult {
    /// The endpoint was triggered and a response was sent
    Triggered,
    /// The notification channel was dropped (e.g., market was dropped)
    ChannelDropped,
}

/// Wrapper object that provides an await function to wait for endpoint trigger
pub struct EndpointCallback {
    receiver: mpsc::Receiver<()>,
}

impl EndpointCallback {
    /// Wait for the endpoint to be triggered with a timeout
    pub async fn wait_for_trigger(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<CallbackResult, anyhow::Error> {
        match tokio::time::timeout(timeout, self.receiver.recv()).await {
            Ok(Some(())) => Ok(CallbackResult::Triggered),
            Ok(None) => Ok(CallbackResult::ChannelDropped),
            Err(_elapsed) => Err(anyhow::anyhow!(
                "Timeout {} waiting for endpoint",
                humantime::format_duration(timeout)
            )),
        }
    }
}

/// Generic wrapper for any response type that can include notification
#[derive(Debug, Clone)]
pub struct WithCallback<T: Display> {
    pub response: T,
    pub callback: Option<mpsc::Sender<()>>,
    pub endpoint_name: String,
    pub call_count: usize,
}

impl<T: Display> WithCallback<T> {
    pub fn new(response: T, sender: mpsc::Sender<()>, endpoint_name: String) -> Self {
        Self {
            response,
            callback: Some(sender),
            endpoint_name,
            call_count: 0,
        }
    }

    /// Increment the call count for this override
    pub fn increment_call_count(&mut self) {
        self.call_count += 1;
    }

    /// Clean up the callback to prevent unwanted message sending
    pub fn cleanup_callback(&mut self) {
        self.callback = None;
    }
}

impl<T: Display> Drop for WithCallback<T> {
    fn drop(&mut self) {
        // Send callback on drop to avoid missed callbacks in case of errors in
        // RCP handlers logic. If this would be done manually, developer could
        // forget to trigger it.
        if let Some(sender) = self.callback.take() {
            log::debug!(
                "{}: sending callback for triggered response: {}",
                self.endpoint_name,
                self.response
            );
            let _ = sender.send(());
        }
    }
}

// Specific implementation for CallOverride types
impl WithCallback<CallOverride> {
    /// Check if this override is already outdated.
    pub fn should_remove_override(&self) -> bool {
        match &self.response {
            CallOverride::Once(_) => true, // Remove after first use
            CallOverride::Until { until, .. } => {
                // Remove if expired
                std::time::Instant::now() >= *until
            }
            CallOverride::NTimes { n, .. } => {
                // Remove if count exceeded
                self.call_count >= *n
            }
        }
    }

    /// Check if this override should be used
    pub fn should_use_override(&self) -> bool {
        match &self.response {
            CallOverride::Once(_) => true,
            CallOverride::Until { until, .. } => std::time::Instant::now() < *until,
            CallOverride::NTimes { n, .. } => self.call_count < *n,
        }
    }
}

/// Response types that can be forced for RPC calls.
#[derive(Debug, derive_more::Display, Clone)]
pub enum CallResponse {
    /// Return a specific error.
    /// TODO: We need to decide for a specific error type or make a wrapper that could handle multiple types.
    Error(String),
    /// Execute normal `subscribe_offer` logic (don't force any response).
    /// This variant allows to capture the fact of calling the RPC. User code can
    /// wait for this event to happen in test or validate this fact as a condition for test to pass.
    Success,
}

#[derive(Debug, derive_more::Display, Clone)]
pub enum CallOverride {
    #[display("Override: once -> {}", _0)]
    Once(CallResponse),
    #[display("Override: until {until:?} -> {response}")]
    Until {
        response: CallResponse,
        until: std::time::Instant,
    },
    #[display("Override: {n} times -> {response}")]
    NTimes { response: CallResponse, n: usize },
}

/// Inner state of the mock controller
#[derive(Debug, Default)]
struct MockControllerInner {
    /// All overrides organized by key, with "global" having priority over RPC-specific keys.
    overrides: HashMap<String, Vec<WithCallback<CallOverride>>>,
}

impl MockControllerInner {
    /// Get the first valid override for a given key (RPC name or "global")
    /// Returns None if no valid override is found
    fn get_first_valid_override(&mut self, key: &str) -> Option<WithCallback<CallOverride>> {
        if let Some(queue) = self.overrides.get_mut(key) {
            if let Some(wrapper) = queue.first_mut() {
                if wrapper.should_use_override() {
                    wrapper.increment_call_count();
                    return Some(wrapper.clone());
                }
            }
        }
        None
    }

    /// Clean up all outdated entries from overrides
    fn cleanup_outdated_overrides(&mut self) {
        for queue in self.overrides.values_mut() {
            queue.retain_mut(|override_wrapper| {
                if override_wrapper.should_remove_override() {
                    // Clean up callback to prevent unwanted message sending
                    override_wrapper.cleanup_callback();
                    false // Remove this entry
                } else {
                    true // Keep this entry
                }
            });
        }
    }
}

/// Controller for managing mock market responses
#[derive(Debug, Default, Clone)]
pub struct MockController {
    inner: Arc<Mutex<MockControllerInner>>,
}

impl MockController {
    /// Create a new mock controller
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a global override response that will be used for any RPC call
    /// Returns a notifier that can be used to wait for the endpoint to be triggered
    pub async fn global_override(&self, rpc_override: CallOverride) -> EndpointCallback {
        let (sender, receiver) = mpsc::channel(CALLBACK_CHANNEL_SIZE);
        let overrides = WithCallback::new(rpc_override, sender, GLOBAL_OVERRIDE_KEY.to_string());

        let mut lock = self.inner.lock().unwrap();
        lock.overrides
            .entry(GLOBAL_OVERRIDE_KEY.to_string())
            .or_insert_with(Vec::new)
            .push(overrides);

        EndpointCallback { receiver }
    }

    /// Add a response override for a specific RPC call
    /// Returns a notifier that can be used to wait for the endpoint to be triggered
    pub async fn override_rpc(
        &self,
        rpc_name: &str,
        rpc_override: CallOverride,
    ) -> EndpointCallback {
        let (sender, receiver) = mpsc::channel(CALLBACK_CHANNEL_SIZE);
        let overrides = WithCallback::new(rpc_override, sender, rpc_name.to_string());

        let mut lock = self.inner.lock().unwrap();
        lock.overrides
            .entry(rpc_name.to_string())
            .or_insert_with(Vec::new)
            .push(overrides);

        EndpointCallback { receiver }
    }

    /// Get the next override response for a specific RPC call (prioritizes global overrides)
    /// Handles Once, Until, and NTimes logic internally
    pub fn take_next_override(&self, rpc_name: &str) -> Option<WithCallback<CallOverride>> {
        let mut controller = self.inner.lock().unwrap();

        // First, clean up all outdated entries. This way in next step we will have
        // a list of overrides that are still valid and could be applied.
        controller.cleanup_outdated_overrides();

        // Global overrides have priority.
        if let Some(mut override_wrapper) = controller.get_first_valid_override(GLOBAL_OVERRIDE_KEY)
        {
            // User can know on which endpoint global override was triggered.
            override_wrapper.endpoint_name = format!("{GLOBAL_OVERRIDE_KEY}: {rpc_name}");
            return Some(override_wrapper);
        }

        // Then check RPC-specific overrides
        if let Some(override_wrapper) = controller.get_first_valid_override(rpc_name) {
            return Some(override_wrapper);
        }

        None
    }
}
