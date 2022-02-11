use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::{ Royalty };
use crate::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub nft_contract_address: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Place an NFT on Auction
    PlaceListing {
        id: String,
        minimum_bid: Asset,
    },
    // Bid on an NFT already put on Auction
    BidListing {
        listing_id: String,
        bid_price: Asset
    },
    // Withdraw an ended Auction
    WithdrawListing {
        listing_id: String,
    },
    Mint(GFMintMsg),
    // register the whitelisted minter or update the expiration time
    UpdateMinter {
        minter: String,
    },
    // remove the minter from whitelist
    RemoveMinter {
        minter: String,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    // Resolve listing returns all the details of a listing
    ResolveListing { id: String },
    // query minters
    QueryMinter {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GFMintMsg{
    pub owner: String,
    // Identifies the asset to which this NFT represents
    pub name: String,
    // A URI pointing to an image representing the asset
    pub image_uri: Option<String>,
    // An external URI
    pub external_link: Option<String>,
    // Describes the asset to which this NFT represents (may be empty)
    pub description: Option<String>,
    // A collection this NFT belongs to
    pub collection: Option<Uint128>,
    // # of real piece representations
    pub num_real_repr: Uint128,
    // # of collectible nfts
    pub num_nfts: Uint128,
    // royalties
    pub royalties: Vec<Royalty>,
    // initial ask price
    pub init_price: Uint128
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ResolveListingResponse {
    pub token_id: String,

    pub contract_addr: Addr,

    pub seller: Addr,

    pub max_bid: Asset,

    pub max_bidder: Addr,

    pub block_limit: u64,
}
