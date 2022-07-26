#[macro_export]
macro_rules! log {
    ($msg:literal) => {
        println!("{:?}: {}", std::thread::current().id(), $msg)
    };
    ($msg:expr $(,)?) => {
        println!("{:?}: {}", std::thread::current().id(), $msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        println!("{:?}: {}", std::thread::current().id(), format!($fmt, $($arg)*))
    };
}
