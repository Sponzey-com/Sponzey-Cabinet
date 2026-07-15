use std::cell::Cell;

use cabinet_domain::webhook::{EventEnvelope, WebhookDestination, WebhookSignature};
use cabinet_ports::webhook::{WebhookDeliveryPortError, WebhookTransportPort};

#[derive(Debug)]
pub struct FakeWebhookTransport {
    result: Result<(), WebhookDeliveryPortError>,
    call_count: Cell<usize>,
}

impl FakeWebhookTransport {
    pub const fn succeeding() -> Self {
        Self {
            result: Ok(()),
            call_count: Cell::new(0),
        }
    }

    pub const fn failing(error: WebhookDeliveryPortError) -> Self {
        Self {
            result: Err(error),
            call_count: Cell::new(0),
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count.get()
    }
}

impl WebhookTransportPort for FakeWebhookTransport {
    fn send_event(
        &self,
        _event: &EventEnvelope,
        _destination: &WebhookDestination,
        _signature: &WebhookSignature,
    ) -> Result<(), WebhookDeliveryPortError> {
        self.call_count.set(self.call_count.get() + 1);
        self.result
    }
}
