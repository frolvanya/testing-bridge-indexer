use bridge_indexer_types::documents_types::{
    OmniEvent, OmniEventData, OmniMetaEvent, OmniMetaEventDetails, OmniTokenInfo,
    OmniTransactionEvent, OmniTransactionOrigin, OmniTransferMessage, OmniTransferStatus,
};
use bson::{doc, oid::ObjectId};
use omni_types::{OmniAddress, TransferId};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

impl From<OmniEventOld> for OmniEvent {
    fn from(event: OmniEventOld) -> Self {
        OmniEvent {
            id: event.id,
            transaction_id: event.get_transaction_id(),
            origin: event.get_origin(),
            event: match event.event {
                OmniEventDataOld::Transaction(tx) => {
                    OmniEventData::Transaction(OmniTransactionEvent {
                        transfer_message: tx.transfer_message.clone(),
                        sender: tx.get_sender(),
                        transfer_id: tx.transfer_id,
                        status: tx.status,
                        enrichment_data:
                            bridge_indexer_types::documents_types::OmniEnrichmentData::from(
                                if event.enrichment_data.is_none() {
                                    tx.enrichment_data
                                } else {
                                    event.enrichment_data
                                },
                            ),
                    })
                }
                OmniEventDataOld::Meta(meta) => OmniEventData::Meta(OmniMetaEvent {
                    details: meta.details,
                }),
            },
        }
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OmniEventOld {
    #[serde(rename = "_id")]
    pub id: Option<ObjectId>,
    #[serde(flatten)]
    pub event: OmniEventDataOld,
    #[serde(default, skip_serializing_if = "OmniEnrichmentData::is_none")]
    pub enrichment_data: OmniEnrichmentData,
}

impl OmniEventOld {
    fn get_transaction_id(&self) -> String {
        match &self.event {
            OmniEventDataOld::Transaction(event) => event.transaction_id.clone(),
            OmniEventDataOld::Meta(event) => event.transaction_id.clone(),
        }
    }

    fn get_origin(&self) -> OmniTransactionOrigin {
        match &self.event {
            OmniEventDataOld::Transaction(event) => event.origin.clone(),
            OmniEventDataOld::Meta(event) => event.origin.clone(),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OmniEventDataOld {
    Transaction(OmniTransactionEventOld),
    Meta(OmniMetaEventOld),
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

impl From<OmniEnrichmentData> for bridge_indexer_types::documents_types::OmniEnrichmentData {
    fn from(data: OmniEnrichmentData) -> Self {
        match data {
            OmniEnrichmentData::None => Self::None,
            OmniEnrichmentData::NotApplicable => Self::NotApplicable,
            OmniEnrichmentData::Data {
                transferred_token_info,
                native_token_info,
            } => Self::Data {
                transferred_token_info,
                native_token_info,
            },
        }
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OmniTransactionEventOld {
    pub transfer_message: OmniTransferMessage,
    pub transaction_id: String,
    pub origin: OmniTransactionOrigin,
    pub sender: Option<OmniAddress>,
    pub transfer_id: TransferId,
    pub status: OmniTransferStatus,

    #[serde(default, skip_serializing_if = "OmniEnrichmentData::is_none")]
    pub enrichment_data: OmniEnrichmentData,
}

impl OmniTransactionEventOld {
    fn get_sender(&self) -> Option<OmniAddress> {
        match &self.transfer_message {
            OmniTransferMessage::NearTransferMessage(transfer_message) => {
                Some(transfer_message.sender.clone())
            }
            OmniTransferMessage::NearSignTransferEvent(_near_sign_transfer_event) => None,
            OmniTransferMessage::NearClaimFeeEvent(transfer_message) => {
                Some(transfer_message.sender.clone())
            }
            OmniTransferMessage::EvmInitTransferMessage(init_transfer_message) => {
                Some(init_transfer_message.sender.clone())
            }
            OmniTransferMessage::EvmFinTransferMessage(_fin_transfer_message) => None,
            OmniTransferMessage::SolanaInitTransfer(solana_init_transfer_message) => {
                Some(solana_init_transfer_message.sender.clone())
            }
            OmniTransferMessage::SolanaFinTransfer(_solana_fin_transfer_message) => None,
        }
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OmniMetaEventOld {
    pub transaction_id: String,
    pub origin: OmniTransactionOrigin,
    pub details: OmniMetaEventDetails,
}

impl OmniEnrichmentData {
    fn is_none(&self) -> bool {
        matches!(self, OmniEnrichmentData::None)
    }
}
