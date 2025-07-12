use wikibase_rest_api::prelude::*;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), RestApiError> {
    // #lizard forgives the complexity

    // Use the Wikidata API
    let api = RestApi::wikidata()?;

    // Use Q42 as an example item
    let id_q42 = EntityId::new("Q42")?;

    // Get the label and sitelink of Q42
    let q42_label_en = Label::get(&id_q42, "en", &api).await?.value().to_owned();
    let q42_sitelink = Sitelink::get(&id_q42, "enwiki", &api)
        .await?
        .title()
        .to_owned();
    println!("Q42 '{q42_label_en}' => [[enwiki:{q42_sitelink}]]");

    // Get the statements of Q42
    let statements = Statements::get(&id_q42, &api).await?;
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
