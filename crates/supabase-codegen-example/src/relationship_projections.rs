use crate::public::tables;

rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CustomerIdentity for crate::public::tables::customers { id, name }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct OrderCustomer for crate::public::tables::orders {
        id,
        customer: embed(crate::public::tables::orders::relationships::orders_customer, CustomerIdentity),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CountrySummary for crate::public::tables::countries { id, name }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct AddressSummary for crate::public::tables::addresses {
        id, label,
        country: embed(crate::public::tables::addresses::relationships::address_country, CountrySummary),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct DetailSummary for crate::public::tables::order_details { order_id, note }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct PreferenceSummary for crate::public::tables::customer_preferences { id, label }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct OrderSummary for crate::public::tables::orders {
        id, label,
        billing: embed(crate::public::tables::orders::relationships::orders_billing, AddressSummary),
        shipping: embed(crate::public::tables::orders::relationships::orders_shipping, AddressSummary),
        detail: embed(crate::public::tables::orders::relationships::order_details_details_order, DetailSummary),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct OrderInner for crate::public::tables::orders {
        id,
        billing: embed(crate::public::tables::orders::relationships::orders_billing, AddressSummary, inner),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CustomerSummary for crate::public::tables::customers {
        id, name,
        orders: embed(crate::public::tables::customers::relationships::orders_orders_customer, OrderSummary),
        preference: embed(crate::public::tables::customers::relationships::customer_preferences_preferences_customer, PreferenceSummary),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CustomerInner for crate::public::tables::customers {
        id,
        orders: embed(crate::public::tables::customers::relationships::orders_orders_customer, OrderSummary, inner),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CustomerPredicates for crate::public::tables::customers {
        id,
        matching_orders: empty(crate::public::tables::customers::relationships::orders_orders_customer),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct OrderPredicates for crate::public::tables::orders {
        id,
        matching_details: empty(crate::public::tables::orders::relationships::order_details_details_order),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CompositeParentSummary for crate::public::tables::composite_parents {
        tenant_id, id, label
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CompositeChildSummary for crate::public::tables::composite_children {
        id,
        parent: embed(crate::public::tables::composite_children::relationships::composite_parent, CompositeParentSummary),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CompositeLeaf for crate::public::tables::composite_children { id }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    pub struct CompositeParentChildren for crate::public::tables::composite_parents {
        id,
        children: embed(crate::public::tables::composite_parents::relationships::composite_children_composite_parent, CompositeLeaf),
    }
}

pub fn offline() -> Result<(), Box<dyn std::error::Error>> {
    use rp_supabase_client::schema::Projection;
    let customer: CustomerSummary = serde_json::from_value(serde_json::json!({
        "id": 1, "name": "offline customer", "preference": null,
        "orders": [{"id": 2, "label": "offline order", "detail": null,
            "billing": {"id": 3, "label": "billing", "country": {"id": 4, "name": "country"}},
            "shipping": null}]
    }))?;
    let _: &Vec<OrderSummary> = &customer.orders;
    let _: &Option<PreferenceSummary> = &customer.preference;
    let _: &Option<AddressSummary> = &customer.orders[0].billing;
    let _: &Option<DetailSummary> = &customer.orders[0].detail;
    let predicate: CustomerPredicates = serde_json::from_value(serde_json::json!({"id": 1}))?;
    assert_eq!(predicate.id, customer.id);
    let request = tables::customers::query(rp_supabase_client::rp_postgrest::Postgrest::new(
        "http://localhost",
    )?)
    .select::<CustomerSummary>()
    .embedded(
        CustomerSummary::orders.then(OrderSummary::billing),
        |address| {
            address.eq(tables::addresses::columns::label, "billing");
        },
    )
    .exists(CustomerSummary::orders)
    .into_raw();
    // Constructing the typed request does not contact the server.
    drop(request);
    println!(
        "Offline nested relationship selection: {}",
        CustomerSummary::selection()
    );
    Ok(())
}
