use crate::appender::Appender;
use tracing::Level;

pub trait LayerConfig: Send + Sync {
    fn enabled(&self, level: &Level, module: &str) -> bool;
    fn get_appenders(&self, level: &Level, module: &str) -> Vec<Box<dyn Appender>>;
}
