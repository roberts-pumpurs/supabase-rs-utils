//! Named scalar and relationship projections.
/// Define a named result shape from generated columns and typed relationships.
#[macro_export]
macro_rules! projection {
    ($($input:tt)*) => {
        $crate::schema::__private::__projection! { [$crate] $($input)* }
    };
}
