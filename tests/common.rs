#[macro_export]
macro_rules! lookup_and_assert {
    ($state:expr, $(($var:expr, $expected:expr)),* $(,)?) => {
        $(
            let value = lookup(
                $var,
                $state.env_id,
                &$state.envs,
                &$state.parent,
                &std::collections::BTreeSet::<&str>::new(),
            )
            .unwrap();
            assert_eq!(value, $expected);
        )*
    };
}
