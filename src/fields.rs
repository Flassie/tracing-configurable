use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use tracing::field::{debug, Field, Visit};

pub enum EventValue {
    F64(f64),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    Bool(bool),
    String(String),
}

impl Display for EventValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EventValue::F64(v) => write!(f, "{}", v),
            EventValue::I64(v) => write!(f, "{}", v),
            EventValue::U64(v) => write!(f, "{}", v),
            EventValue::I128(v) => write!(f, "{}", v),
            EventValue::U128(v) => write!(f, "{}", v),
            EventValue::Bool(v) => write!(f, "{}", v),
            EventValue::String(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Default)]
pub struct FieldsVisitor {
    message: Option<String>,
    values: HashMap<&'static str, Vec<EventValue>>,
}

impl FieldsVisitor {
    pub fn message(&self) -> &str {
        self.message.as_deref().unwrap_or("")
    }

    pub fn has_values(&self) -> bool {
        !self.values.is_empty()
    }

    pub fn format_values(&self) -> String {
        self.values
            .iter()
            .filter_map(|(key, values)| {
                if values.len() > 1 {
                    let values = values
                        .iter()
                        .map(|i| match i {
                            EventValue::String(v) => format!("`{}`", v),
                            v => format!("{}", v),
                        })
                        .collect::<Vec<_>>()
                        .join(",");

                    Some(format!("{}=[{}]", key, values))
                } else if let Some(v) = values.get(0) {
                    let v = match v {
                        EventValue::String(v) => format!("`{}`", v),
                        v => format!("{}", v),
                    };

                    Some(format!("{}={}", key, v))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl Visit for FieldsVisitor {
    fn record_f64(&mut self, field: &Field, value: f64) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(value.to_string())
        } else {
            self.values
                .entry(field.name())
                .or_default()
                .push(EventValue::F64(value))
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(value.to_string())
        } else {
            self.values
                .entry(field.name())
                .or_default()
                .push(EventValue::I64(value))
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(value.to_string())
        } else {
            self.values
                .entry(field.name())
                .or_default()
                .push(EventValue::U64(value))
        }
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(value.to_string())
        } else {
            self.values
                .entry(field.name())
                .or_default()
                .push(EventValue::I128(value))
        }
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(value.to_string())
        } else {
            self.values
                .entry(field.name())
                .or_default()
                .push(EventValue::U128(value))
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(value.to_string())
        } else {
            self.values
                .entry(field.name())
                .or_default()
                .push(EventValue::Bool(value))
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(value.to_string())
        } else {
            self.values
                .entry(field.name())
                .or_default()
                .push(EventValue::String(value.to_string()))
        }
    }

    fn record_error(&mut self, field: &Field, value: &(dyn Error + 'static)) {
        self.record_debug(field, &debug(value))
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" && self.message.is_none() {
            self.message = Some(format!("{:?}", value))
        } else {
            self.values
                .entry(field.name())
                .or_default()
                .push(EventValue::String(format!("{:?}", value)))
        }
    }
}
