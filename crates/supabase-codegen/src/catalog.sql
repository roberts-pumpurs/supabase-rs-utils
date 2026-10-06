-- Static catalog-only queries. Sections are executed separately in one snapshot transaction.
-- query: schemas
SELECT nspname::text AS name FROM pg_catalog.pg_namespace
WHERE nspname::text = ANY($1::text[]) ORDER BY nspname;
-- query: types
SELECT t.oid::bigint AS oid, n.nspname::text AS schema, t.typname::text AS name,
       t.typtype::text AS kind, t.typbasetype::bigint AS base,
       t.typelem::bigint AS element, t.typarray::bigint AS array,
       t.typrelid::bigint AS relation, t.typnotnull AS not_null,
       (t.typdefaultbin IS NOT NULL OR t.typdefault IS NOT NULL) AS has_default
FROM pg_catalog.pg_type t
JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
WHERE t.typisdefined ORDER BY n.nspname, t.typname;
-- query: enums
SELECT enumtypid::bigint AS oid, enumlabel::text AS label
FROM pg_catalog.pg_enum ORDER BY enumtypid, enumsortorder;
-- query: attributes
SELECT a.attrelid::bigint AS relation, a.attname::text AS name,
       a.atttypid::bigint AS type_oid, a.attnotnull AS not_null,
       a.attidentity::text AS identity, a.attgenerated::text AS generated,
       (d.oid IS NOT NULL AND a.attgenerated = '') AS has_default
FROM pg_catalog.pg_attribute a
LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
WHERE a.attnum > 0 AND NOT a.attisdropped
ORDER BY a.attrelid, a.attnum;
-- query: relations
SELECT n.nspname::text AS schema, c.relname::text AS name,
       c.oid::bigint AS oid, c.relkind::text AS kind
FROM pg_catalog.pg_class c
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname::text = ANY($1::text[]) AND c.relkind IN ('r', 'p', 'f', 'v', 'm')
ORDER BY n.nspname, c.relname;
-- query: functions
SELECT n.nspname::text AS schema, p.proname::text AS name,
       COALESCE(p.proallargtypes, p.proargtypes::oid[])::bigint[] AS types,
       p.proargmodes::text[] AS modes, p.proargnames AS names,
       p.pronargs::integer AS input_count, p.pronargdefaults::integer AS defaults,
       p.prorettype::bigint AS return_oid, p.proretset AS returns_set
FROM pg_catalog.pg_proc p
JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
WHERE n.nspname::text = ANY($1::text[]) AND p.prokind = 'f'
ORDER BY n.nspname, p.proname, ARRAY(
    SELECT ns.nspname::text || '.' || t.typname::text
    FROM pg_catalog.unnest(p.proargtypes::oid[]) WITH ORDINALITY AS arg(oid, position)
    JOIN pg_catalog.pg_type t ON t.oid = arg.oid
    JOIN pg_catalog.pg_namespace ns ON ns.oid = t.typnamespace
    ORDER BY arg.position
);
