//! Load a whole item in a single request and print a summary of its parts.
//!
//! Run with: `cargo run --example read_entity`
use wikibase_rest_api::prelude::*;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), RestApiError> {
    // #lizard forgives the complexity
    let api = RestApi::wikidata()?;

    // One request fetches the entire item.
    let item = Item::get(EntityId::new("Q42")?, &api).await?;
    println!("Item {}", item.id());

    // Labels / descriptions are keyed by language code.
    if let Some(label) = item.labels().get_lang("en") {
        println!("  English label:       {label}");
    }
    if let Some(description) = item.descriptions().get_lang("en") {
        println!("  English description: {description}");
    }

    // Aliases return a list per language.
    let en_aliases = item.aliases().get_lang("en");
    if !en_aliases.is_empty() {
        println!("  English aliases:     {}", en_aliases.join(", "));
    }

    // Counts across all languages / properties.
    println!(
        "  {} labels, {} descriptions, {} statements, {} sitelinks",
        item.labels().len(),
        item.descriptions().len(),
        item.statements().len(),
        item.sitelinks().len(),
    );

    // "instance of" (P31) statements point at other items.
    for statement in item.statements().property("P31") {
        if let StatementValue::Value(StatementValueContent::String(other_id)) = statement.value() {
            println!("  instance of:         {other_id}");
        }
    }

    // The English Wikipedia sitelink, if any.
    if let Some(sitelink) = item.sitelinks().get_wiki("enwiki") {
        println!("  enwiki:              {}", sitelink.title());
    }

    Ok(())
}
