use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, WasmMsg, Uint128, Decimal256,
};

use crate::coin_helpers::assert_sent_sufficient_coin;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ResolveListingResponse, GFMintMsg};
use crate::state::{store_config, read_config, store_minters, remove_minter, read_minters, read_minter_info, list_resolver, list_resolver_read, Config, Listing, MinterInfo, Metadata};
use cw721::{
    Cw721ExecuteMsg::{Approve, TransferNft},
    Expiration,
};

use cw721_base::msg::{ ExecuteMsg as Cw721ExecuteMsg, MintMsg };
pub const DEFAULT_EXPIRATION: u64 = 1000000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    let config_state = Config { 
        listing_count: 0,
        owner: info.sender.to_string(),
        expiration_time: DEFAULT_EXPIRATION,
        nft_contract_address: deps.api.addr_validate(&msg.nft_contract_address)?,
    };
    // Initiate listing_id with 0
    store_config(deps.storage, &config_state)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Route messages to appropriate handlers
        ExecuteMsg::PlaceListing {
            nft_contract_address,
            id,
            minimum_bid,
        } => execute_place_listing(deps, env, info, nft_contract_address, id, minimum_bid),
        ExecuteMsg::BidListing { listing_id } => execute_bid_listing(deps, env, info, listing_id),
        ExecuteMsg::WithdrawListing { listing_id } => {
            execute_withdraw_listing(deps, env, info, listing_id)
        },
        ExecuteMsg::Mint(mint_msg) => execute_mint(deps, env, info, mint_msg),
        ExecuteMsg::UpdateMinter{ minter } => update_minters(deps, env, info, &minter),
        ExecuteMsg::RemoveMinter{ minter } => unregister_minter(deps, env, info, &minter),
    }
}

fn update_minters(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    minter: &String
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;
    let owner = deps.api.addr_validate(&config.owner)?;

    if info.sender != owner {
        return Err(ContractError::Unauthorized{});
    }

    let minter_info = MinterInfo {
        expiration_time: DEFAULT_EXPIRATION
    };

    store_minters(deps.storage, deps.api.addr_validate(minter)?, minter_info)?;
    Ok(Response::default())
}

fn unregister_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    minter: &String
) -> Result<Response, ContractError> {
    let config = read_config(deps.storage)?;
    let owner = deps.api.addr_validate(&config.owner)?;

    if info.sender != owner{
        return Err(ContractError::Unauthorized{});
    }

    remove_minter(deps.storage, deps.api.addr_validate(minter)?)?;
    Ok(Response::default())
}

fn execute_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: GFMintMsg,
) -> Result<Response, ContractError> {
    // check if the sender is a whitelisted minter
    let minter_info = read_minter_info(deps.storage, info.sender);

    if minter_info.expiration_time == 0 {
        return Err(ContractError::Unauthorized{});
    }

    // check if royalties are set properly. sum of them must not be greater than 100%
    let mut sum_total_rate = Decimal256::zero();

    for royalty in msg.royalties.iter() {
        sum_total_rate = sum_total_rate + (*royalty).royalty_rate;
    }

    if sum_total_rate > Decimal256::one() {
        return Err(ContractError::InvalidRoyaltyRate {})
    }

    let mut config = read_config(deps.storage)?;
    config.listing_count = config.listing_count + 1;

    store_config(deps.storage, &config)?;

    let token_id: String = ["GF".to_string(), config.listing_count.to_string()].join(".");

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.nft_contract_address.to_string(),
            msg: to_binary(&Cw721ExecuteMsg::Mint(MintMsg {
                token_id,
                owner: msg.owner,
                token_uri: msg.image_uri,
                extension: Metadata {
                    name: msg.name,
                    description: msg.description,
                    external_link: msg.external_link,
                    collection: Some(Uint128::from(1 as u128)),
                    num_real_repr: msg.num_real_repr,
                    num_nfts:msg.num_nfts,
                    royalties: msg.royalties,
                    init_price: msg.init_price
                }
            }))?,
            funds: vec![]
        }))
    )
}

pub fn execute_bid_listing(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    listing_id: String,
) -> Result<Response, ContractError> {
    // Fetch listing from listing_id
    let key = listing_id.as_bytes();
    let mut listing = list_resolver_read(deps.storage).load(key)?;
    if listing.block_limit < _env.block.height {
        return Err(ContractError::AuctionEnded {});
    }

    // check if current bid exceeds the previous one
    let sent_coin = assert_sent_sufficient_coin(&info.funds, listing.max_bid.clone())?;
    let last_bid = listing.max_bid;
    let last_bidder = listing.max_bidder;

    // update bidder
    listing.max_bidder = info.sender.clone();
    listing.max_bid = sent_coin;
    list_resolver(deps.storage).save(key, &listing)?;

    if _env.contract.address != last_bidder {
        // return money to last bidder
        Ok(Response::new()
            .add_attribute("Bidding", listing_id)
            .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: last_bidder.to_string(),
                amount: vec![last_bid.unwrap()],
            })))
    } else {
        // no need to return money since first bid
        Ok(Response::new().add_attribute("Bidding", listing_id))
    }
}

pub fn execute_place_listing(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    nft_contract_address: String,
    id: String,
    minimum_bid: Option<Coin>,
) -> Result<Response, ContractError> {
    // update listing id in store
    let config_state = read_config(deps.storage)?;
    let listing_count = config_state.listing_count + 1;
    let nft_contract = deps.api.addr_validate(&nft_contract_address)?;

    // Each auction has a limit for 50000 blocks
    let listing = Listing {
        token_id: id.clone(),
        contract_addr: nft_contract,
        seller: info.sender.clone(),
        max_bid: minimum_bid,
        max_bidder: _env.contract.address.clone(),
        block_limit: _env.block.height + 50000,
    };

    let key = listing_count.to_string();
    // save listing to store
    list_resolver(deps.storage).save(key.as_bytes(), &listing)?;

    // lock nft to contract
    Ok(Response::new()
        .add_attribute("place_listing", id.to_string())
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: nft_contract_address.clone(),
                funds: vec![],
                msg: to_binary(&Approve {
                    spender: _env.contract.address.to_string(),
                    token_id: id.clone(),
                    expires: Some(Expiration::AtHeight(_env.block.height + 20000)),
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: nft_contract_address,
                funds: vec![],
                msg: to_binary(&TransferNft {
                    recipient: String::from(_env.contract.address.as_str()),
                    token_id: id,
                })?,
            }),
        ]))
}

pub fn execute_withdraw_listing(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    listing_id: String,
) -> Result<Response, ContractError> {
    let key = listing_id.as_bytes();
    let listing = list_resolver_read(deps.storage).load(key)?;

    // Check if the auction ended or not
    if listing.block_limit >= _env.block.height {
        return Err(ContractError::AuctionNotEnded {});
    }
    // remove listing from the store
    list_resolver(deps.storage).remove(key);

    // If noone has put a bid then then seller will be sent back with his NFT
    // Transfer the locked NFT to highest bidder and bid amount to the seller
    if _env.contract.address != listing.max_bidder {
        Ok(Response::new()
            .add_attribute("listing_sold", listing_id.to_string())
            .add_messages(vec![
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: listing.contract_addr.to_string(),
                    funds: vec![],
                    msg: to_binary(&TransferNft {
                        recipient: listing.max_bidder.to_string(),
                        token_id: listing_id.clone(),
                    })?,
                }),
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: listing.max_bidder.to_string(),
                    amount: vec![listing.max_bid.unwrap()],
                }),
            ]))
    } else {
        Ok(Response::new()
            .add_attribute("listing_unsold", listing_id.to_string())
            .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: listing.contract_addr.to_string(),
                funds: vec![],
                msg: to_binary(&TransferNft {
                    recipient: listing.seller.to_string(),
                    token_id: listing_id.clone(),
                })?,
            })]))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&read_config(deps.storage)?),
        QueryMsg::ResolveListing { id } => query_list_resolver(deps, env, id),
        QueryMsg::QueryMinter {} => to_binary(&query_minters(deps, env)?),
    }
}

pub fn query_minters(deps: Deps, _env: Env) -> StdResult<Vec<String>> {
    read_minters(deps.storage)  
}

fn query_list_resolver(deps: Deps, _env: Env, id: String) -> StdResult<Binary> {
    // Fetch listing from listing_id
    let key = id.as_bytes();

    let resp = match list_resolver_read(deps.storage).may_load(key)? {
        Some(listing) => Some(listing),
        None => None,
    };
    let unwrapped_resp = resp.unwrap();
    let resolve_listing = ResolveListingResponse {
        token_id: unwrapped_resp.token_id,
        contract_addr: unwrapped_resp.contract_addr,
        seller: unwrapped_resp.seller,
        max_bid: unwrapped_resp.max_bid,
        max_bidder: unwrapped_resp.max_bidder,
        block_limit: unwrapped_resp.block_limit,
    };
    to_binary(&resolve_listing)
}
