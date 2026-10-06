// Adapted from rp-postgrest 2.1.0 (MIT), Copyright (c) 2020 Supabase.
use crate::Builder;

macro_rules! scalar_filters {
    ($( $(#[$meta:meta])* $name:ident => $operator:literal; )*) => {
        impl Builder {
            $(
                $(#[$meta])*
                pub fn $name<T, U>(mut self, column: T, filter: U) -> Self
                where
                    T: AsRef<str>,
                    U: AsRef<str>,
                {
                    self.append_query(column.as_ref().to_owned(), format!(concat!($operator, ".{}"), filter.as_ref()));
                    self
                }
            )*
        }
    };
}

scalar_filters! {
    /// Matches equality. The scalar is literal: no quoting or escaping is added.
    eq => "eq";
    /// Matches inequality. The scalar is literal: no quoting or escaping is added.
    neq => "neq";
    /// Matches values greater than the literal scalar.
    gt => "gt";
    /// Matches values greater than or equal to the literal scalar.
    gte => "gte";
    /// Matches values less than the literal scalar.
    lt => "lt";
    /// Matches values less than or equal to the literal scalar.
    lte => "lte";
    /// Matches a case-sensitive SQL pattern, preserving `%`, `*`, and backslashes.
    like => "like";
    /// Matches a case-insensitive SQL pattern, preserving `%`, `*`, and backslashes.
    ilike => "ilike";
    /// Matches `null`, `true`, or `false` using `PostgREST`'s `is` operator.
    is => "is";
    /// Matches containment using caller-provided JSON, array, or range grammar.
    cs => "cs";
}

macro_rules! owned_column_filters {
    ($( $(#[$meta:meta])* $name:ident => $operator:literal; )*) => {
        impl Builder {
            $(
                $(#[$meta])*
                pub fn $name<T, U>(mut self, column: T, filter: U) -> Self
                where
                    T: Into<String>,
                    U: AsRef<str>,
                {
                    self.append_query(column.into(), format!(concat!($operator, ".{}"), filter.as_ref()));
                    self
                }
            )*
        }
    };
}

owned_column_filters! {
    /// Matches contained-by using caller-provided JSON, array, or range grammar.
    cd => "cd";
    /// Matches overlap using caller-provided array or range grammar.
    ov => "ov";
}

macro_rules! range_filters {
    ($( $(#[$meta:meta])* $name:ident => $operator:literal; )*) => {
        impl Builder {
            $(
                $(#[$meta])*
                pub fn $name<T>(mut self, column: T, range: (i64, i64)) -> Self
                where
                    T: Into<String>,
                {
                    self.append_query(column.into(), format!(concat!($operator, ".({},{})"), range.0, range.1));
                    self
                }
            )*
        }
    };
}

range_filters! {
    /// Matches ranges strictly left of the supplied exclusive-bound range.
    sl => "sl";
    /// Matches ranges strictly right of the supplied exclusive-bound range.
    sr => "sr";
    /// Matches ranges that do not extend left of the supplied range.
    nxl => "nxl";
    /// Matches ranges that do not extend right of the supplied range.
    nxr => "nxr";
    /// Matches ranges adjacent to the supplied range.
    adj => "adj";
}

macro_rules! text_filters {
    ($( $(#[$meta:meta])* $name:ident => $operator:literal; )*) => {
        impl Builder {
            $(
                $(#[$meta])*
                pub fn $name<T, U>(mut self, column: T, tsquery: U, config: Option<&str>) -> Self
                where
                    T: Into<String>,
                    U: AsRef<str>,
                {
                    let value = match config {
                        Some(config) => format!(concat!($operator, "({}).{}"), config, tsquery.as_ref()),
                        None => format!(concat!($operator, ".{}"), tsquery.as_ref()),
                    };
                    self.append_query(column.into(), value);
                    self
                }
            )*
        }
    };
}

text_filters! {
    /// Uses `to_tsquery`; query and optional configuration remain caller grammar.
    fts => "fts";
    /// Uses `plainto_tsquery`; query and optional configuration remain caller grammar.
    plfts => "plfts";
    /// Uses `phraseto_tsquery`; query and optional configuration remain caller grammar.
    phfts => "phfts";
    /// Uses `websearch_to_tsquery`; query and optional configuration remain caller grammar.
    wfts => "wfts";
}

impl Builder {
    /// Negates a raw operator and filter without changing their grammar.
    pub fn not<T, U, V>(mut self, operator: T, column: U, filter: V) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
        V: AsRef<str>,
    {
        self.append_query(
            column.as_ref().to_owned(),
            format!("not.{}.{}", operator.as_ref(), filter.as_ref()),
        );
        self
    }

    /// Conjoins raw filter grammar, adding only the outer parentheses.
    /// Quote reserved characters within the grammar yourself; do not URL-encode it.
    pub fn and<T: AsRef<str>>(mut self, filters: T) -> Self {
        self.append_query("and", format!("({})", filters.as_ref()));
        self
    }

    /// Disjoins raw filter grammar, adding only the outer parentheses.
    /// Quote reserved characters within the grammar yourself; do not URL-encode it.
    pub fn or<T: AsRef<str>>(mut self, filters: T) -> Self {
        self.append_query("or", format!("({})", filters.as_ref()));
        self
    }

    /// Matches a list of raw `PostgREST` grammar fragments.
    ///
    /// Elements are comma-separated without escaping or additional quoting.
    /// For example, `["\"Paris,France\"", "null"]` retains the quoted first
    /// fragment and the unquoted second fragment. Use [`Self::in_values`] for
    /// literal text elements instead.
    pub fn in_<T, U, V>(mut self, column: T, values: U) -> Self
    where
        T: AsRef<str>,
        U: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let mut filter = String::from("in.(");
        for (index, value) in values.into_iter().enumerate() {
            if index != 0 {
                filter.push(',');
            }
            filter.push_str(value.as_ref());
        }
        filter.push(')');
        self.append_query(column.as_ref().to_owned(), filter);
        self
    }

    /// Matches literal text elements, quoting every element in list grammar.
    ///
    /// Double quotes and backslashes are escaped within each quoted element.
    /// Commas, parentheses, periods, empty strings, and text such as `null`
    /// remain part of that element rather than becoming list grammar. This
    /// helper does not quote column identifiers or URL-encode either input.
    pub fn in_values<T, U, V>(mut self, column: T, values: U) -> Self
    where
        T: AsRef<str>,
        U: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let mut filter = String::from("in.(");
        for (index, value) in values.into_iter().enumerate() {
            if index != 0 {
                filter.push(',');
            }
            filter.push('"');
            for character in value.as_ref().chars() {
                if matches!(character, '"' | '\\') {
                    filter.push('\\');
                }
                filter.push(character);
            }
            filter.push('"');
        }
        filter.push(')');
        self.append_query(column.as_ref().to_owned(), filter);
        self
    }
}
