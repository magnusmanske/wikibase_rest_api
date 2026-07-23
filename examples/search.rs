//! Search Wikidata for items matching a text query.
//!
//! Run with: `cargo run --example search`
use wikibase_rest_api::prelude::*;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Search on Wikidata currently requires API version 0.
    let api = RestApi::builder("https://www.wikidata.org/w/rest.php")?
        .with_api_version(0)
        .build()?;

    let language = Language::try_new("en")?;
    let query = "Douglas Adams";

    let results = Search::items(query, language).get(&api).await?;
    println!("Top results for '{query}':");
    for result in results.iter().take(10) {
        let label = result
            .display_label()
            .map(|l| l.value())
            .unwrap_or_default();
        let description = result.description().map(|d| d.value()).unwrap_or_default();
        println!("  {:10} {label} — {description}", result.id());
    }

    Ok(())
}
