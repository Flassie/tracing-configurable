#![allow(dead_code)]

use crate::config::LayerConfig;
use crate::fields::FieldsVisitor;
use crate::renderer::EventRenderer;
use tracing::span::{Attributes, Id};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub mod appender;
pub mod config;
pub mod fields;
pub mod pattern;
pub mod renderer;

struct ConfigurableLayer {
    config: Box<dyn LayerConfig>,
}

impl<S> Layer<S> for ConfigurableLayer
where
    S: Subscriber + for<'l> LookupSpan<'l>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut fields = FieldsVisitor::default();
        attrs.record(&mut fields);

        ctx.span(id)
            .expect("span not found")
            .extensions_mut()
            .replace(fields); // can be `insert`, but `insert` can panic
    }

    fn event_enabled(&self, event: &Event<'_>, _: Context<'_, S>) -> bool {
        self.config
            .enabled(event.metadata().level(), event.metadata().target())
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let appenders = self
            .config
            .get_appenders(event.metadata().level(), event.metadata().target());
        for appender in appenders {
            let pattern = appender.pattern();
            if let Some(v) = pattern.render(event, &ctx) {
                appender.write(&v)
            }
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        ctx.span(id)
            .expect("span not found")
            .extensions_mut()
            .remove::<FieldsVisitor>();
    }
}

#[cfg(test)]
mod test {
    use crate::appender::Appender;
    use crate::pattern::Pattern;
    use crate::{ConfigurableLayer, LayerConfig};
    use std::io::{stdout, Write};
    use tracing::{error, info, trace_span, Level};
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::registry;
    use tracing_subscriber::util::SubscriberInitExt;

    #[test]
    fn test() {
        struct StdoutAppender {
            pattern: Pattern,
        }

        impl Appender for StdoutAppender {
            fn pattern(&self) -> &Pattern {
                &self.pattern
            }

            fn write(&self, value: &str) {
                let _ = writeln!(stdout().lock(), "{}", value);
                // let _ = stdout().lock().write(value.as_bytes());
            }
        }

        struct TestConfig {}

        impl LayerConfig for TestConfig {
            fn enabled(&self, level: &Level, module: &str) -> bool {
                true
            }

            fn get_appenders(&self, level: &Level, module: &str) -> Vec<Box<dyn Appender>> {
                vec![Box::new(StdoutAppender {
                    pattern: Pattern::try_parse(
                        "$level(width = 5, alignment = '>') $datetime $target$fields(prefix = '{', suffix = '}')$span(prefix = '::', args, args_prefix ='{', args_suffix = '}'): $message",
                    )
                        .unwrap(),
                })]
            }
        }

        registry()
            .with(ConfigurableLayer {
                config: Box::new(TestConfig {}),
            })
            .init();

        let test = trace_span!("test", arg = 1, arg = "test").entered();
        info!(test = "123", "Hello, world!");
        error!("test error");
    }
}
