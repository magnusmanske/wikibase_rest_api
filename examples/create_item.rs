use wikibase_rest_api::prelude::*;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<(), RestApiError> {
    // #lizard forgives the complexity
    // let token = "MY_ACCESS_TOKEN";
    let api = RestApi::builder("https://test.wikidata.org/w/rest.php")?
        // .with_access_token(token)
        .build();
    let mut item = Item::default();
    item.labels_mut()
        .insert(LanguageString::new("en", "My label"));
    item.descriptions_mut()
        .insert(LanguageString::new("en", "My description123"));
    item.statements_mut()
        .insert(Statement::new_string("P31", "Q13406268"));
    let item = item.post(&api).await?;
    println!(
        "Created new item https://test.wikidata.org/wiki/{}",
        item.id()
    );

    Ok(())
}
