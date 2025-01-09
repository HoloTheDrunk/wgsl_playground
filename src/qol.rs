macro_rules! map {
    {$($key:expr => $value:expr),* $(,)?} => {
        ::std::collections::HashMap::from([$(($key, $value)),*])
    };
}
pub(crate) use map;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn map() {
        map! {
            "test" => 42,

        };
    }
}
