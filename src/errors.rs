#[macro_export]
macro_rules! itry {
    ($x: expr) => {
        match $x {
            Err(e) => return Some(Err(From::from(e))),
            Ok(v) => v,
        }
    }
}
