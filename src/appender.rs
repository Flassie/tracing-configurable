use crate::pattern::Pattern;

pub trait Appender {
    fn pattern(&self) -> &Pattern;
    fn write(&self, value: &str);
}
