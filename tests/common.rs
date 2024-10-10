#[macro_export]
macro_rules! lookup_and_assert {
    ($state:expr, $(($var:expr, $expected:expr)),* $(,)?) => {
        $(
            let value = lookup($var, &$state.local_env, &$state.global_env, &$state.store).unwrap();
            assert_eq!(*value, $expected);
        )*
    };
}
