use bridge_indexer_types::documents_types::{OmniMetaEvent, OmniTransactionEvent};
use bson::{from_bson, Bson, Document};
use futures::stream::StreamExt;
use mongodb::{options::ClientOptions, Client, Collection};
use serde::{de::DeserializeOwned, Serialize};

async fn watch_collections<T>(collection: Collection<Document>)
where
    T: DeserializeOwned + Serialize + std::fmt::Debug,
{
    let mut stream = collection.watch().await.unwrap();
    while let Some(change) = stream.next().await {
        match change {
            Ok(doc) => {
                if let Some(full_document) = doc.full_document {
                    match from_bson::<T>(Bson::Document(full_document)) {
                        Ok(json) => {
                            println!("Change detected:\n{:?}", json);
                        }
                        Err(e) => {
                            eprintln!("Failed to parse document: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => eprintln!("Error watching changes: {}", e),
        }
    }
}

#[tokio::main]
async fn main() -> mongodb::error::Result<()> {
    let uri = std::env::var("MONGO_URI").unwrap();

    let client_options = ClientOptions::parse(uri).await?;
    let client = Client::with_options(client_options)?;

    let db = client.database("testnet_omni_bridge_db");
    let omni_transactions_collection = db.collection::<bson::Document>("omni_transactions");
    let omni_meta_events_collection = db.collection::<bson::Document>("omni_meta_events");

    let handles = vec![
        tokio::spawn(watch_collections::<OmniTransactionEvent>(
            omni_transactions_collection,
        )),
        tokio::spawn(watch_collections::<OmniMetaEvent>(
            omni_meta_events_collection,
        )),
    ];

    futures::future::join_all(handles).await;

    Ok(())
}
