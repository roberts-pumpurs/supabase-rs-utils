use crate::{public::tables, relationship_projections::*};
use rp_supabase_client::{postgrest::Postgrest, schema::Field};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned().into())
    }
}

pub async fn live(client: Postgrest) -> Result<()> {
    // A unique run label scopes cleanup even after an HTTP/decode failure during insertion.
    let base = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos(),
    )?;
    let tag = format!("relationship-smoke-{base}");
    let scenario = async {
        tables::countries::query(client.clone())
            .insert(&tables::countries::Insert {
                id: base,
                name: tag.clone(),
            })
            .fetch()
            .await?;
        for id in [base, base + 1] {
            tables::addresses::query(client.clone())
                .insert(&tables::addresses::Insert {
                    id,
                    label: tag.clone(),
                    country_id: Field::Value(Some(base)),
                })
                .fetch()
                .await?;
            tables::customers::query(client.clone())
                .insert(&tables::customers::Insert {
                    id,
                    name: tag.clone(),
                })
                .fetch()
                .await?;
        }
        for (id, billing, shipping) in [
            (base, Some(base), Some(base + 1)),
            (base + 1, None, None),
            (base + 2, Some(base + 1), Some(base)),
        ] {
            tables::orders::query(client.clone())
                .insert(&tables::orders::Insert {
                    id,
                    customer_id: base,
                    billing_id: Field::Value(billing),
                    shipping_id: Field::Value(shipping),
                    label: tag.clone(),
                })
                .fetch()
                .await?;
        }
        tables::order_details::query(client.clone())
            .insert(&tables::order_details::Insert {
                order_id: base,
                note: tag.clone(),
            })
            .fetch()
            .await?;
        tables::customer_preferences::query(client.clone())
            .insert(&tables::customer_preferences::Insert {
                id: base,
                customer_id: base,
                label: tag.clone(),
            })
            .fetch()
            .await?;
        // Same parent id in two tenants detects crossed/sorted composite-FK pairs.
        for tenant_id in [base, base + 1] {
            tables::composite_parents::query(client.clone())
                .insert(&tables::composite_parents::Insert {
                    tenant_id,
                    id: base,
                    label: tag.clone(),
                })
                .fetch()
                .await?;
        }
        tables::composite_children::query(client.clone())
            .insert(&tables::composite_children::Insert {
                id: base,
                parent_id: base,
                tenant_id: base + 1,
                label: tag.clone(),
            })
            .fetch()
            .await?;

        let customer = tables::customers::query(client.clone())
            .select::<CustomerSummary>()
            .eq(tables::customers::columns::id, &base)
            .fetch_one()
            .await?;
        require(
            customer.orders.len() == 3,
            "reverse to-many did not return three orders",
        )?;
        require(
            customer.preference.as_ref().is_some_and(|p| p.id == base),
            "UNIQUE-backed reverse relation was not a to-one object",
        )?;
        let first = customer
            .orders
            .iter()
            .find(|o| o.id == base)
            .ok_or("first order missing")?;
        require(
            first.billing.as_ref().is_some_and(|a| a.id == base),
            "billing FK hint failed",
        )?;
        require(
            first.shipping.as_ref().is_some_and(|a| a.id == base + 1),
            "shipping FK hint failed",
        )?;
        require(
            first.detail.as_ref().is_some_and(|d| d.order_id == base),
            "PK-backed reverse relation was not a to-one object",
        )?;
        let direct = tables::orders::query(client.clone())
            .select::<OrderCustomer>()
            .eq(tables::orders::columns::id, &base)
            .fetch_one()
            .await?;
        require(
            direct.customer.as_ref().is_some_and(|c| c.id == base),
            "non-null direct FK did not decode a conservative to-one",
        )?;

        let left = tables::orders::query(client.clone())
            .select::<OrderSummary>()
            .eq(tables::orders::columns::label, tag.as_str())
            .embedded(OrderSummary::billing, |address| {
                address.eq(tables::addresses::columns::id, &base);
            })
            .fetch()
            .await?;
        require(left.len() == 3, "left embed unexpectedly removed parents")?;
        require(
            left.iter().filter(|o| o.billing.is_none()).count() == 2,
            "left embed did not decode filtered/null children as None",
        )?;
        let inner = tables::orders::query(client.clone())
            .select::<OrderInner>()
            .eq(tables::orders::columns::label, tag.as_str())
            .embedded(OrderInner::billing, |address| {
                address.eq(tables::addresses::columns::id, &base);
            })
            .fetch()
            .await?;
        require(
            inner.len() == 1 && inner[0].id == base && inner[0].billing.is_some(),
            "inner embed did not remove unmatched parents",
        )?;

        let left_many = tables::customers::query(client.clone())
            .select::<CustomerSummary>()
            .eq(tables::customers::columns::name, tag.as_str())
            .embedded(CustomerSummary::orders, |orders| {
                orders.eq(tables::orders::columns::id, &base);
            })
            .fetch()
            .await?;
        require(
            left_many.len() == 2 && left_many.iter().any(|c| c.orders.is_empty()),
            "left reverse embed did not retain empty parent with []",
        )?;
        let inner_many = tables::customers::query(client.clone())
            .select::<CustomerInner>()
            .eq(tables::customers::columns::name, tag.as_str())
            .embedded(CustomerInner::orders, |orders| {
                orders.eq(tables::orders::columns::id, &base);
            })
            .fetch()
            .await?;
        require(
            inner_many.len() == 1 && inner_many[0].orders.len() == 1,
            "inner reverse embed did not remove empty parent",
        )?;

        let aliases = tables::orders::query(client.clone())
            .select::<OrderSummary>()
            .eq(tables::orders::columns::id, &base)
            .embedded(OrderSummary::billing, |address| {
                address.eq(tables::addresses::columns::id, &base);
            })
            .embedded(OrderSummary::shipping, |address| {
                address.eq(tables::addresses::columns::id, &(base + 1));
            })
            .embedded(
                OrderSummary::billing.then(AddressSummary::country),
                |country| {
                    country.eq(tables::countries::columns::name, tag.as_str());
                },
            )
            .fetch_one()
            .await?;
        require(
            aliases
                .billing
                .as_ref()
                .and_then(|a| a.country.as_ref())
                .is_some_and(|c| c.id == base)
                && aliases.shipping.is_some(),
            "independent aliases or nested typed path failed",
        )?;
        let nested = tables::customers::query(client.clone())
            .select::<CustomerSummary>()
            .eq(tables::customers::columns::id, &base)
            .embedded(CustomerSummary::orders, |orders| {
                orders.embedded(OrderSummary::billing, |address| {
                    address.eq(tables::addresses::columns::id, &base);
                });
                orders.exists(OrderSummary::billing);
            })
            .fetch_one()
            .await?;
        require(
            nested.orders.len() == 1 && nested.orders[0].id == base,
            "scoped nested existence/filter did not retain matching order",
        )?;

        let exists = tables::customers::query(client.clone())
            .select::<CustomerPredicates>()
            .eq(tables::customers::columns::name, tag.as_str())
            .embedded(CustomerPredicates::matching_orders, |orders| {
                orders.eq(tables::orders::columns::id, &base);
            })
            .exists(CustomerPredicates::matching_orders)
            .fetch()
            .await?;
        let absent = tables::customers::query(client.clone())
            .select::<CustomerPredicates>()
            .eq(tables::customers::columns::name, tag.as_str())
            .not_exists(CustomerPredicates::matching_orders)
            .fetch()
            .await?;
        require(
            exists.len() == 1
                && exists[0].id == base
                && absent.len() == 1
                && absent[0].id == base + 1,
            "empty embed exists/anti-exists failed",
        )?;
        let no_detail = tables::orders::query(client.clone())
            .select::<OrderPredicates>()
            .eq(tables::orders::columns::label, tag.as_str())
            .not_exists(OrderPredicates::matching_details)
            .fetch()
            .await?;
        require(no_detail.len() == 2, "to-one anti-existence failed")?;

        let composite = tables::composite_children::query(client.clone())
            .select::<CompositeChildSummary>()
            .eq(tables::composite_children::columns::id, &base)
            .fetch_one()
            .await?;
        require(
            composite
                .parent
                .as_ref()
                .is_some_and(|p| p.tenant_id == base + 1 && p.id == base),
            "composite FK column pairing failed",
        )?;
        let parents = tables::composite_parents::query(client.clone())
            .select::<CompositeParentChildren>()
            .eq(tables::composite_parents::columns::label, tag.as_str())
            .fetch()
            .await?;
        require(
            parents.len() == 2 && parents.iter().map(|p| p.children.len()).sum::<usize>() == 1,
            "composite reverse cardinality failed",
        )?;
        let write_id = base + 9;
        let inserted = tables::orders::query(client.clone())
            .select::<OrderSummary>()
            .embedded(OrderSummary::billing, |address| {
                address.eq(tables::addresses::columns::id, &base);
            })
            .exists(OrderSummary::billing)
            .insert(&tables::orders::Insert {
                id: write_id,
                customer_id: base,
                billing_id: Field::Value(Some(base)),
                shipping_id: Field::Value(Some(base + 1)),
                label: tag.clone(),
            })
            .fetch_one()
            .await?;
        require(
            inserted.id == write_id
                && inserted
                    .billing
                    .as_ref()
                    .is_some_and(|address| address.id == base)
                && inserted
                    .shipping
                    .as_ref()
                    .is_some_and(|address| address.id == base + 1),
            "locked relationship insert representation failed",
        )?;
        let updated = tables::orders::query(client.clone())
            .select::<OrderSummary>()
            .eq(tables::orders::columns::id, &write_id)
            .embedded(OrderSummary::billing, |address| {
                address.eq(tables::addresses::columns::id, &base);
            })
            .update(&tables::orders::Update {
                shipping_id: Field::Value(None),
                ..tables::orders::Update::default()
            })
            .fetch_one()
            .await?;
        require(
            updated.id == write_id && updated.billing.is_some() && updated.shipping.is_none(),
            "locked relationship update failed to clear its FK",
        )?;
        let deleted = tables::orders::query(client.clone())
            .select::<OrderSummary>()
            .eq(tables::orders::columns::id, &write_id)
            .embedded(OrderSummary::billing, |address| {
                address.eq(tables::addresses::columns::id, &base);
            })
            .is_null(tables::orders::columns::shipping_id)
            .delete()
            .fetch_one()
            .await?;
        require(deleted.id == write_id, "locked relationship delete failed")?;
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    // Never early-return from cleanup: remove dependent rows first and attempt every table.
    let mut cleanup_error = None;
    macro_rules! cleanup {
        ($table:ident, $column:ident) => {
            if let Err(error) = tables::$table::query(client.clone())
                .eq(tables::$table::columns::$column, tag.as_str())
                .delete()
                .fetch()
                .await
            {
                cleanup_error.get_or_insert(error);
            }
        };
    }
    cleanup!(order_details, note);
    cleanup!(customer_preferences, label);
    cleanup!(orders, label);
    cleanup!(composite_children, label);
    cleanup!(composite_parents, label);
    cleanup!(addresses, label);
    cleanup!(customers, name);
    cleanup!(countries, name);
    match (scenario, cleanup_error) {
        (Err(error), Some(cleanup)) => {
            Err(format!("{error}; cleanup also failed: {cleanup}").into())
        }
        (Err(error), None) => Err(error),
        (Ok(()), Some(error)) => Err(error.into()),
        (Ok(()), None) => {
            println!(
                "Live direct/reverse/unique/composite/nested/existence/locked-write relationship scenario passed"
            );
            Ok(())
        }
    }
}
