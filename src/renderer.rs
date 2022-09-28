use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;

pub trait EventRenderer<S: Subscriber> {
    fn render(&self, event: &Event, context: &Context<'_, S>) -> Option<String>;
}
