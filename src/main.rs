use bson::{from_bson, oid::ObjectId, Bson, Document};
use futures::stream::StreamExt;
use mongodb::{bson::doc, options::ClientOptions, Client, Collection};
use near_primitives::types::AccountId;
use near_sdk::json_types::U128;
use omni_types::{
    mpc_types::SignatureResponse,
    prover_result::{
        DeployTokenMessage, FinTransferMessage, InitTransferMessage, LogMetadataMessage,
    },
    BasicMetadata, ChainKind, Fee, MetadataPayload, Nonce, OmniAddress, TransferId,
    TransferMessagePayload,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OmniMetaEventDetails {
    EVMDeployToken(DeployTokenMessage),
    EVMLogMetadata(LogMetadataMessage),
    SolanaDeployToken {
        token: String,
        name: String,
        symbol: String,
        decimals: u8,
        emitter: String,
        sequence: u64,
    },
    SolanaLogMetadata {
        token: String,
        emitter: String,
        sequence: u64,
    },
    NearSignTransferEvent {
        signature: SignatureResponse,
        message_payload: TransferMessagePayload,
    },
    NearLogMetadataEvent {
        signature: SignatureResponse,
        metadata_payload: MetadataPayload,
    },
    NearClaimFeeEvent {
        transfer_message: omni_types::TransferMessage,
    },
    NearDeployTokenEvent {
        token_id: AccountId,
        token_address: OmniAddress,
        metadata: BasicMetadata,
    },
    NearBindTokenEvent {
        token_id: AccountId,
        token_address: OmniAddress,
        decimals: u8,
        origin_decimals: u8,
    },
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OmniMetaEvent {
    #[serde(rename = "_id")]
    pub id: Option<ObjectId>,
    pub transaction_id: String,
    pub origin: OmniTransactionOrigin,
    pub details: OmniMetaEventDetails,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OmniTransactionEvent {
    #[serde(rename = "_id")]
    pub id: Option<ObjectId>,
    pub transfer_message: OmniTransferMessage,
    pub sender: Option<OmniAddress>,
    pub transaction_id: String,
    pub transfer_id: TransferId,
    pub status: OmniTransferStatus,
    pub origin: OmniTransactionOrigin,
    #[serde(default, skip_serializing_if = "OmniEnrichmentData::is_none")]
    pub enrichment_data: OmniEnrichmentData,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OmniTransferMessage {
    NearTransferMessage(omni_types::TransferMessage),
    EvmInitTransferMessage(InitTransferMessage),
    EvmFinTransferMessage(FinTransferMessage),
    SolanaInitTransfer(SolanaInitTransferMessage),
    SolanaFinTransfer(SolanaFinTransferMessage),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OmniTransferStatus {
    Initialized,
    FinalisedOnNear,
    Finalised,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OmniTransactionOrigin {
    NearReceipt {
        raw_receipt_id: Option<ObjectId>,
        block_height: u64,
        block_timestamp_nanosec: u64,
        receipt_id: String,
        contract_id: AccountId,
        signer_id: AccountId,
        predecessor_id: AccountId,
        version: u32,
    },
    EVMLog {
        block_number: u64,
        block_timestamp: u64,
        transaction_index: Option<u64>,
        log_index: Option<u64>,
        chain_kind: ChainKind,
    },
    SolanaTransaction {
        slot: u64,
        block_time: u64,
        instruction_index: usize,
    },
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SolanaInitTransferMessage {
    pub amount: U128,
    pub fee: Fee,
    pub token: OmniAddress,
    pub recipient: OmniAddress,
    pub sender: OmniAddress,
    pub origin_nonce: Nonce,
    pub emitter: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SolanaFinTransferMessage {
    pub amount: U128,
    pub destination_nonce: u64,
    pub fee_recipient: Option<String>,
    pub emitter: Option<String>,
    pub sequence: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct CoingeckoTokenId(pub String);

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OmniTokenInfo {
    pub token_id: CoingeckoTokenId,
    pub name: String,
    pub symbol: String,
    pub decimals: u32,
    pub usd_price: f64,
}

#[skip_serializing_none]
#[derive(Default, Deserialize, Serialize, Debug, Clone)]
pub enum OmniEnrichmentData {
    #[default]
    None,
    NotApplicable,
    Data {
        transferred_token_info: Option<OmniTokenInfo>,
        native_token_info: OmniTokenInfo,
    },
}

impl OmniEnrichmentData {
    fn is_none(&self) -> bool {
        matches!(self, OmniEnrichmentData::None)
    }
}

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
