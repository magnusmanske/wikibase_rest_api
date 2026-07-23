//! Add a statement to an existing item, with an edit summary.
//!
//! Writing requires an OAuth 2.0 access token, so this targets
//! test.wikidata.org. Uncomment the token line and adjust the item/property
//! IDs to ones that exist on your target wiki before running.
//!
//! Run with: `cargo run --example add_statement`
use wikibase_rest_api::prelude::*;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), RestApiError> {
    // let token = "MY_ACCESS_TOKEN";
    let api = RestApi::builder("https://test.wikidata.org/w/rest.php")?
        // .with_access_token(token)
        .with_user_agent("wikibase_rest_api add_statement example")
        .build()?;

    // The item to add a statement to.
    let id = EntityId::new("Q231343")?;

    // Build the statement (adjust property/value to ones valid on your wiki).
    let statement = Statement::new_string("P95201", "example value");

    // Attach an edit summary to the write.
    let mut meta = EditMetadata::default();
    meta.set_comment(Some(
        "Add example statement via wikibase_rest_api".to_string(),
    ));

    // POST the new statement; the server returns the created statement (with its ID).
    let created = Statements::default()
        .post_meta(&id, statement, &api, meta)
        .await?;
    println!("Created statement {:?} on {id}", created.id());

    Ok(())
}
