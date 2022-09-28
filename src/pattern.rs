use crate::fields::FieldsVisitor;
use crate::renderer::EventRenderer;
use chrono::Local;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

#[cfg(feature = "parse")]
use argable_parser::item::{Arg, Item, Value};

pub struct Pattern {
    items: Vec<PatternItem>,
}

impl Pattern {
    pub fn new(items: Vec<PatternItem>) -> Self {
        Self { items }
    }

    #[cfg(feature = "parse")]
    pub fn try_parse<S: AsRef<str>>(str: S) -> Result<Self, anyhow::Error> {
        let items = argable_parser::parse(str.as_ref())?;

        let items = items
            .into_iter()
            .filter_map(|item| match item {
                Item::Text(v) => Some(PatternItem::Text(v)),
                Item::Placeholder(v) => {
                    let ty = PlaceholderType::from_str(v.name).or_else(|| {
                        eprintln!("unknown placeholder type");
                        None
                    })?;

                    let mut properties: HashMap<String, PlaceholderValue> = HashMap::new();
                    let mut flags = Vec::new();

                    if let Some(args) = v.args {
                        for arg in args {
                            match arg {
                                Arg::Flag(v) => {
                                    flags.push(v.to_string());
                                }
                                Arg::Value(name, value) => {
                                    properties.insert(name.to_string(), value.into());
                                }
                            }
                        }
                    }

                    Some(PatternItem::Placeholder(Placeholder {
                        ty,
                        properties,
                        flags,
                    }))
                }
            })
            .collect();

        Ok(Self::new(items))
    }

    pub fn items(&self) -> &[PatternItem] {
        &self.items
    }

    pub fn into_inner(self) -> Vec<PatternItem> {
        self.items
    }
}

impl<S> EventRenderer<S> for Pattern
where
    S: Subscriber + for<'l> LookupSpan<'l>,
{
    fn render(&self, event: &Event, context: &Context<'_, S>) -> Option<String> {
        thread_local! {
            static BUF: RefCell<String> = RefCell::new(String::new())
        }

        let v = BUF.with(|buf| {
            let mut buf = buf.borrow_mut();

            for item in self.items() {
                let parent_span = Lazy::new(|| {
                    event
                        .parent()
                        .and_then(|i| context.span(i))
                        .or_else(|| context.lookup_current())
                });

                let fields = Lazy::new(|| {
                    let mut fields = FieldsVisitor::default();
                    event.record(&mut fields);
                    fields
                });

                match item {
                    PatternItem::Text(v) => {
                        let _ = write!(buf, "{}", v);
                    }
                    PatternItem::Placeholder(placeholder) => {
                        let inner: Option<Cow<str>> = match placeholder.ty {
                            PlaceholderType::Text => placeholder.str("value").map(Cow::Borrowed),
                            PlaceholderType::Target => {
                                Some(Cow::Borrowed(event.metadata().target()))
                            }
                            PlaceholderType::Level => {
                                Some(Cow::Borrowed(event.metadata().level().as_str()))
                            }
                            PlaceholderType::File => event.metadata().file().map(Cow::Borrowed),
                            PlaceholderType::Line => {
                                event.metadata().line().map(|i| Cow::Owned(i.to_string()))
                            }
                            PlaceholderType::Span => {
                                let v = parent_span.as_ref().map(|i| {
                                    let name = i.metadata().name();
                                    let extensions = i.extensions();
                                    let fields = extensions.get::<FieldsVisitor>();

                                    if fields.is_some() && placeholder.flag("args") {
                                        let fields = fields.as_ref().unwrap().format_values();
                                        (name, Some(fields))
                                    } else {
                                        (name, None)
                                    }
                                });

                                if let Some((name, fields)) = &v {
                                    if fields.is_some() && placeholder.flag("args") {
                                        let prefix = placeholder.str("args_prefix").unwrap_or("");
                                        let suffix = placeholder.str("args_suffix").unwrap_or("");

                                        Some(Cow::Owned(format!(
                                            "{}{}{}{}",
                                            name,
                                            prefix,
                                            fields.as_ref().unwrap(),
                                            suffix
                                        )))
                                    } else {
                                        Some(Cow::Borrowed(name))
                                    }
                                } else {
                                    None
                                }
                            }
                            PlaceholderType::Message => Some(Cow::Borrowed(fields.message())),
                            PlaceholderType::Fields => {
                                if fields.has_values() {
                                    Some(Cow::Owned(fields.format_values()))
                                } else {
                                    None
                                }
                            }
                            PlaceholderType::DateTime => {
                                let now = Local::now();
                                let now = if let Some(fmt) = placeholder.str("fmt") {
                                    now.format(fmt)
                                } else {
                                    now.format("%Y-%m-%d %H:%M:%S%.6f")
                                };

                                Some(Cow::Owned(now.to_string()))
                            }
                        };

                        if let Some(value) = inner {
                            if let Some(prefix) = placeholder.str("prefix") {
                                let _ = write!(buf, "{}", prefix);
                            }

                            let width = placeholder.int("width").map(|i| i as usize);
                            let is_left_align = placeholder.str("alignment").and_then(|i| {
                                if i.eq_ignore_ascii_case("<") {
                                    Some(true)
                                } else if i.eq_ignore_ascii_case(">") {
                                    Some(false)
                                } else {
                                    None
                                }
                            });

                            if width.is_some() {
                                let _ = match is_left_align {
                                    Some(true) => {
                                        write!(buf, "{:<width$}", value, width = width.unwrap())
                                    }
                                    Some(false) => {
                                        write!(buf, "{:>width$}", value, width = width.unwrap())
                                    }
                                    None => write!(buf, "{}", value),
                                };
                            } else {
                                let _ = write!(buf, "{}", value);
                            }

                            if let Some(suffix) = placeholder.str("suffix") {
                                let _ = write!(buf, "{}", suffix);
                            }
                        }
                    }
                }
            }

            let ret = buf.to_string();
            buf.clear();

            ret
        });

        Some(v)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub enum PatternItem {
    Text(String),
    Placeholder(Placeholder),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub enum PlaceholderValue {
    String(String),
    Boolean(bool),
    Integer(i32),
    Float(f32),
}

#[cfg(feature = "parse")]
impl From<Value> for PlaceholderValue {
    fn from(v: Value) -> Self {
        match v {
            Value::String(str) => PlaceholderValue::String(str),
            Value::Integer(v) => PlaceholderValue::Integer(v),
            Value::Float(v) => PlaceholderValue::Float(v),
            Value::Boolean(v) => PlaceholderValue::Boolean(v),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Placeholder {
    ty: PlaceholderType,
    properties: HashMap<String, PlaceholderValue>,
    flags: Vec<String>,
}

impl Placeholder {
    pub fn new(
        ty: PlaceholderType,
        props: HashMap<String, PlaceholderValue>,
        flags: Vec<String>,
    ) -> Self {
        Self {
            ty,
            properties: props,
            flags,
        }
    }

    pub fn ty(&self) -> &PlaceholderType {
        &self.ty
    }

    pub fn str<N: AsRef<str>>(&self, name: N) -> Option<&str> {
        self.property(name).and_then(|i| {
            if let PlaceholderValue::String(str) = i {
                Some(str.as_str())
            } else {
                None
            }
        })
    }

    pub fn bool<N: AsRef<str>>(&self, name: N) -> Option<bool> {
        self.property(name).and_then(|i| {
            if let PlaceholderValue::Boolean(v) = i {
                Some(*v)
            } else {
                None
            }
        })
    }

    pub fn int<N: AsRef<str>>(&self, name: N) -> Option<i32> {
        self.property(name).and_then(|i| {
            if let PlaceholderValue::Integer(v) = i {
                Some(*v)
            } else {
                None
            }
        })
    }

    pub fn float<N: AsRef<str>>(&self, name: N) -> Option<f32> {
        self.property(name).and_then(|i| {
            if let PlaceholderValue::Float(v) = i {
                Some(*v)
            } else {
                None
            }
        })
    }

    pub fn property<N: AsRef<str>>(&self, name: N) -> Option<&PlaceholderValue> {
        self.properties.get(name.as_ref())
    }

    pub fn flag<F: AsRef<str>>(&self, flag: F) -> bool {
        self.flags.iter().any(|i| i == flag.as_ref())
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u16)]
pub enum PlaceholderType {
    Text = 1,
    Message = 2,
    Span = 3,
    Target = 4,
    Level = 5,
    File = 6,
    Line = 7,
    Fields = 8,
    DateTime = 9,
}

impl PlaceholderType {
    pub fn from_str<S: AsRef<str>>(v: S) -> Option<Self> {
        let v = v.as_ref().to_lowercase();

        match v.as_str() {
            "text" => Some(Self::Text),
            "message" => Some(Self::Message),
            "span" => Some(Self::Span),
            "target" => Some(Self::Target),
            "level" => Some(Self::Level),
            "file" => Some(Self::File),
            "line" => Some(Self::Line),
            "fields" => Some(Self::Fields),
            "datetime" => Some(Self::DateTime),
            _ => None,
        }
    }
}
