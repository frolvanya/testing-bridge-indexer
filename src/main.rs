use std::io::Write;

use bridge_indexer_types::documents_types::OmniEvent;
use bson::doc;
use futures::stream::StreamExt;
use mongodb::{options::ClientOptions, Client, Database};

use types::OmniEventOld;

mod types;

async fn migrate(db: Database, events: Vec<OmniEvent>) {
    print!("Enter the new collection name: ");
    std::io::stdout().flush().unwrap();
    let mut new_collection_name = String::new();
    std::io::stdin()
        .read_line(&mut new_collection_name)
        .unwrap();

    print!("Do you want to make a migration? (y/n): ");

    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    if input.trim() != "y" {
        println!("Migration aborted.");
        return;
    }

    println!("Migrating...");

    db.collection::<OmniEvent>(new_collection_name.trim())
        .insert_many(events)
        .await
        .unwrap();
}

#[tokio::main]
async fn main() -> mongodb::error::Result<()> {
    let uri = std::env::var("MONGO_URI").unwrap();

    let client_options = ClientOptions::parse(uri).await?;
    let client = Client::with_options(client_options)?;

    let db = client.database("testnet_omni_bridge_db");
    let collection = db.collection::<OmniEventOld>("omni_events");
    let mut cursor = collection.find(doc! {}).await?;

    let mut new_events = Vec::new();

    while let Some(result) = cursor.next().await {
        match result {
            Ok(event) => {
                println!("Before: {:?}", event);

                let after = OmniEvent::from(event);
                println!("After: {:?}", after);
                println!();

                new_events.push(after);
            }
            Err(e) => {
                panic!("Error: {}", e);
            }
        }
    }

    println!("Total events: {}", new_events.len());

    migrate(db, new_events).await;

    Ok(())
}
