# Supabase schema bindings

This vocabulary describes schema-derived data and PostgREST selections.

## Language

**Column contract**:
The readable value and permitted write values of a SQL column, including nullability, omission, and database defaults.

**Projection**:
A fixed selection of columns and related rows for one relation, together with its response shape. A selected null value differs from a missing selected field.
