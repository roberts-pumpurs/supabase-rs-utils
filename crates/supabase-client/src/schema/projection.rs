//! Named scalar and relationship projections.
/// Define a named result shape from generated columns and typed relationships.
#[macro_export]
macro_rules! projection {
    ($(#[$attribute:meta])* $visibility:vis struct $name:ident for $($table:ident)::+ { $($input:tt)* }) => {
        $crate::projection!(@parse [$(#[$attribute])*] [$visibility] [$name] [$($table)::+] [] [] [] [] $($input)* ,);
    };
    (@parse $attrs:tt $vis:tt $name:tt $table:tt $fields:tt $selections:tt $handles:tt $checks:tt , $($rest:tt)*) => {
        $crate::projection!(@parse $attrs $vis $name $table $fields $selections $handles $checks $($rest)*);
    };
    (@parse $attrs:tt $vis:tt [$name:ident] [$($table:ident)::+] [$($fields:tt)*] [$($selections:tt)*] [$($handles:tt)*] [$($checks:tt)*] $field:ident : embed($edge:path, $child:ty $(, $inner:ident)?), $($rest:tt)*) => {
        $crate::projection!(@parse $attrs $vis [$name] [$($table)::+]
            [$($fields)* ($field [<<$edge as $crate::schema::Relationship>::Cardinality as $crate::schema::Cardinality>::Output<$child>] [$crate::schema::__private::alias(stringify!($field))])]
            [$($selections)* (embed $field [$edge] [$child] [$($inner)?])]
            [$($handles)* #[allow(non_upper_case_globals)] pub const $field: $crate::schema::Embed<$name, $child, $edge> = $crate::schema::Embed::new($crate::schema::__private::alias(stringify!($field)));]
            [$($checks)* $crate::schema::__private::check_embed::<$($table)::+::Row, $edge, $child>();]
            $($rest)*);
    };
    (@parse $attrs:tt $vis:tt [$name:ident] [$($table:ident)::+] [$($fields:tt)*] [$($selections:tt)*] [$($handles:tt)*] [$($checks:tt)*] $field:ident : empty($edge:path), $($rest:tt)*) => {
        $crate::projection!(@parse $attrs $vis [$name] [$($table)::+]
            [$($fields)*] [$($selections)* (empty $field [$edge])]
            [$($handles)* #[allow(non_upper_case_globals)] pub const $field: $crate::schema::Embed<$name, $crate::schema::EmptySelection<<$edge as $crate::schema::Relationship>::Target>, $edge> = $crate::schema::Embed::new($crate::schema::__private::alias(stringify!($field)));]
            [$($checks)* $crate::schema::__private::check_empty::<$($table)::+::Row, $edge>();]
            $($rest)*);
    };
    (@parse $attrs:tt $vis:tt $name:tt [$($table:ident)::+] [$($fields:tt)*] [$($selections:tt)*] $handles:tt $checks:tt $field:ident, $($rest:tt)*) => {
        $crate::projection!(@parse $attrs $vis $name [$($table)::+]
            [$($fields)* ($field [<$($table)::+::columns::$field as $crate::schema::Column>::Value] [<$($table)::+::columns::$field as $crate::schema::Column>::NAME])]
            [$($selections)* (scalar [$($table)::+::columns::$field])]
            $handles $checks $($rest)*);
    };
    (@parse [$($attrs:tt)*] [$vis:vis] [$name:ident] $table:tt [$(($field:ident [$ty:ty] [$key:expr]))*] [$($selection:tt)*] [$($handles:tt)*] [$($checks:tt)*]) => {
        $($attrs)*
        $vis struct $name { $(pub $field: $ty,)* }
        impl $name { $($handles)* }
        const _: () = {
            $($checks)*
            $crate::schema::__private::assert_distinct(&[$($crate::projection!(@key $selection)),*]);
        };
        impl $crate::schema::Projection for $name {
            type Relation = $crate::projection!(@row $table);
            const SELECT_LEN: usize = (0usize $(+ $crate::projection!(@len $selection) + 1usize)*).saturating_sub(1);
            fn write_selection(__output: &mut ::std::string::String) {
                let __start = __output.len();
                $(if __output.len() != __start { __output.push(','); }
                $crate::projection!(@write __output $selection);)*
            }
        }
        impl<'de> $crate::schema::__private::serde::Deserialize<'de> for $name {
            fn deserialize<__D>(__deserializer: __D) -> ::core::result::Result<Self, __D::Error>
            where __D: $crate::schema::__private::serde::Deserializer<'de> {
                struct __Key(::core::option::Option<&'static str>);
                impl<'de> $crate::schema::__private::serde::Deserialize<'de> for __Key {
                    fn deserialize<__K>(__deserializer: __K) -> ::core::result::Result<Self, __K::Error>
                    where __K: $crate::schema::__private::serde::Deserializer<'de> {
                        struct __KeyVisitor;
                        impl<'de> $crate::schema::__private::serde::de::Visitor<'de> for __KeyVisitor {
                            type Value = __Key;
                            fn expecting(&self, __formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result { __formatter.write_str("a projection field name") }
                            fn visit_str<__E>(self, __key: &str) -> ::core::result::Result<Self::Value, __E>
                            where __E: $crate::schema::__private::serde::de::Error {
                                $(if __key == $key { return Ok(__Key(Some($key))); })*
                                Ok(__Key(None))
                            }
                        }
                        __deserializer.deserialize_identifier(__KeyVisitor)
                    }
                }
                struct __Visitor;
                impl<'de> $crate::schema::__private::serde::de::Visitor<'de> for __Visitor {
                    type Value = $name;
                    fn expecting(&self, __formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result { __formatter.write_str(concat!("projection ", stringify!($name))) }
                    fn visit_map<__M>(self, mut __map: __M) -> ::core::result::Result<Self::Value, __M::Error>
                    where __M: $crate::schema::__private::serde::de::MapAccess<'de> {
                        $(let mut $field: Option<$ty> = None;)*
                        while let Some(__Key(__key)) = __map.next_key::<__Key>()? {
                            $(if __key == Some($key) {
                                if $field.is_some() { return Err(<__M::Error as $crate::schema::__private::serde::de::Error>::duplicate_field($key)); }
                                $field = Some(__map.next_value()?); continue;
                            })*
                            let _: $crate::schema::__private::serde::de::IgnoredAny = __map.next_value()?;
                        }
                        Ok($name { $($field: $field.ok_or_else(|| <__M::Error as $crate::schema::__private::serde::de::Error>::missing_field($key))?,)* })
                    }
                }
                __deserializer.deserialize_map(__Visitor)
            }
        }
    };
    (@row [$($table:ident)::+]) => { $($table)::+::Row };
    (@key (scalar [$column:path])) => { <$column as $crate::schema::Column>::NAME };
    (@key (embed $field:ident [$edge:path] [$child:ty] $inner:tt)) => { $crate::schema::__private::alias(stringify!($field)) };
    (@key (empty $field:ident [$edge:path])) => { $crate::schema::__private::alias(stringify!($field)) };
    (@inner_len []) => { 0usize };
    (@inner_len [inner]) => { 6usize };
    (@inner_write $output:ident []) => {};
    (@inner_write $output:ident [inner]) => { $output.push_str("!inner"); };
    (@len (scalar [$column:path])) => { <$column as $crate::schema::Column>::SELECT.len() };
    (@len (embed $field:ident [$edge:path] [$child:ty] $inner:tt)) => {
        $crate::schema::__private::identifier_len($crate::schema::__private::alias(stringify!($field))) + <$edge as $crate::schema::Relationship>::RESOURCE.len() + <$edge as $crate::schema::Relationship>::HINT.len() + <$child as $crate::schema::Projection>::SELECT_LEN + 4usize + $crate::projection!(@inner_len $inner)
    };
    (@len (empty $field:ident [$edge:path])) => {
        $crate::schema::__private::identifier_len($crate::schema::__private::alias(stringify!($field))) + <$edge as $crate::schema::Relationship>::RESOURCE.len() + <$edge as $crate::schema::Relationship>::HINT.len() + 4usize
    };
    (@write $output:ident (scalar [$column:path])) => { $output.push_str(<$column as $crate::schema::Column>::SELECT); };
    (@write $output:ident (embed $field:ident [$edge:path] [$child:ty] $inner:tt)) => {
        $crate::projection!(@head $output $field [$edge]);
        $crate::projection!(@inner_write $output $inner);
        $output.push('('); <$child as $crate::schema::Projection>::write_selection($output); $output.push(')');
    };
    (@write $output:ident (empty $field:ident [$edge:path])) => { $crate::projection!(@head $output $field [$edge]); $output.push_str("()"); };
    (@head $output:ident $field:ident [$edge:path]) => {
        $crate::schema::__private::write_identifier($output, $crate::schema::__private::alias(stringify!($field)));
        $output.push(':'); $output.push_str(<$edge as $crate::schema::Relationship>::RESOURCE);
        $output.push('!'); $output.push_str(<$edge as $crate::schema::Relationship>::HINT);
    };
}
