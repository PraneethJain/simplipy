#[macro_export]
macro_rules! lookup_and_assert {
    ($state:expr, $(($var:expr, $expected:expr)),* $(,)?) => {
        $(
            let value = lookup(
                $var,
                $state.stack.last().and_then(|x| Some(x.1)).unwrap_or(0),
                &$state.envs,
                &$state.parent,
                &std::collections::BTreeSet::<&str>::new(),
            )
            .unwrap();
            assert_eq!(value, $expected);
        )*
    };
}
