use std::sync::Arc;
use wikibase_rest_api::prelude::*;

#[cfg(not(tarpaulin_include))]
async fn q42_demo() -> Result<(), RestApiError> {
    // #lizard forgives the complexity
    let api = RestApi::builder("https://www.wikidata.org/w/rest.php")?.build();

    // Use Q42 as an example item
    let id = EntityId::new("Q42")?;

    // Get the label and sitelink of Q42
    let q42_label_en = Label::get(&id, "en", &api).await?.value().to_owned();
    let q42_sitelink = Sitelink::get(&id, "enwiki", &api).await?.title().to_owned();
    println!("Q42 '{q42_label_en}' => [[enwiki:{q42_sitelink}]]");

    // What is Q42?
    let statements = Statements::get(&id, &api).await?;
    for statement in statements.property("P31") {
        if let StatementValue::Value(StatementValueContent::String(id)) = statement.value() {
            let label = Label::get(&EntityId::Item(id.to_owned()), "en", &api)
                .await?
                .value()
                .to_owned();
            println!("{q42_label_en} ([[Q42]]) is a {label} ([[{id}]])");
        }
    }

    Ok(())
}

#[cfg(not(tarpaulin_include))]
async fn container_demo() -> Result<(), RestApiError> {
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

#[cfg(not(tarpaulin_include))]
async fn create_item_demo() -> Result<(), RestApiError> {
    // #lizard forgives the complexity
    let token = "MY_ACCESS_TOKEN";
    let api = RestApi::builder("https://test.wikidata.org/w/rest.php")?
        .with_access_token(token)
        .build();
    let mut item = Item::default();
    item.labels_mut()
        .insert(LanguageString::new("en", "My label"));
    item.descriptions_mut()
        .insert(LanguageString::new("en", "My description"));
    item.statements_mut()
        .insert(Statement::new_string("P31", "Q42"));
    let item = item.post(&api).await?;
    println!("Created new item {}", item.id());
    Ok(())
}

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<(), RestApiError> {
    q42_demo().await?;

    container_demo().await?;

    create_item_demo().await?;

    Ok(())
}
