# tracing-configurable

Example:
```rust
fn main() {
    struct StdoutAppender {
        pattern: Pattern,
    }

    impl Appender for StdoutAppender {
        fn pattern(&self) -> &Pattern {
            &self.pattern
        }

        fn write(&self, value: &str) {
            let _ = writeln!(stdout().lock(), "{}", value);
        }
    }

    struct TestConfig {}

    impl LayerConfig for TestConfig {
        fn enabled(&self, _: &Level, _: &str) -> bool {
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
```

Example output:
```
 INFO 2022-09-28 13:26:29.379797 tracing_configurable::test{test=`123`}::test{arg=[1,`test`]}: Hello, world!
ERROR 2022-09-28 13:26:29.380502 tracing_configurable::test::test{arg=[1,`test`]}: test error
```