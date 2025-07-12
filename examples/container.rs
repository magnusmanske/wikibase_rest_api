use std::sync::Arc;
use wikibase_rest_api::prelude::*;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), RestApiError> {
    // #lizard forgives the complexity
    let api = RestApi::builder("https://www.wikidata.org/w/rest.php")?.build();
    let api = Arc::new(api);

    // Load several items at once
    // Try to load some items, and a property; Q6, Q7, and Q9 do not exist though.
    // They will be silently ignored.
    let entity_ids = [
        "Q42", "Q1", "Q2", "Q3", "Q4", "Q5", "Q6", "Q7", "Q8", "Q9", "P214",
    ]
    .iter()
    .map(|id| EntityId::new(*id))
    .collect::<Result<Vec<_>, RestApiError>>()?;

    let entity_container = EntityContainer::builder()
        .api(api)
        .max_concurrent(5)
        .build()?;
    println!("Trying to load {} items&properties", entity_ids.len());
    entity_container.load(&entity_ids).await?;
    println!(
        "Loaded {} items",
        entity_container.items().read().await.len()
    );
    println!(
        "Loaded {} properties",
        entity_container.properties().read().await.len()
    );
    println!(
        "Items loaded: {:?}",
        entity_container.items().read().await.keys()
    );
    println!(
        "Properties loaded: {:?}",
        entity_container.properties().read().await.keys()
    );

    // Access item info from the container
    let q42 = entity_container
        .items()
        .read()
        .await
        .get("Q42")
        .ok_or_else(|| RestApiError::IsNone)?
        .to_owned();
    let q42_label_en = q42
        .labels()
        .get_lang("en")
        .ok_or_else(|| RestApiError::IsNone)?;
    println!("Container item Q42 label is '{q42_label_en}'");

    Ok(())
}
