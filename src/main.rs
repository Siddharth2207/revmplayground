
use ethers::contract::BaseContract; 
use ethers::abi::parse_abi;
use ethers::providers::{Http, Provider};
use revm::primitives::Bytes;
use revm::{
    db::{CacheDB, EmptyDB, EthersDB},
    primitives::{address, ExecutionResult, Output, TransactTo, U256},
    Database, EVM,
};
use std::str::FromStr;
use std::sync::Arc;
use std::convert::TryFrom;
use revm::primitives::keccak256;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // create ethers client and wrap it in Arc<M>
    let client = Provider::<Http>::try_from(
        "https://polygon.llamarpc.com",
    )?;
    let client = Arc::new(client);

    // Balance Of mapping is at index 0. so slot index is keccak(0)
    let slot_index = "18569430475105882587588266137607568536673111973893317399460219858819262702947".parse().unwrap();

    // ERC20 contract address
    let pool_address = address!("84342e932797fc62814189f01f0fb05f52519708"); 

    // generate abi for the calldata from the human readable interface
    let abi = BaseContract::from(
        parse_abi(&[
            "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
            "function balanceOf(address account) external view returns (uint256)",
            "function totalSupply() external view returns (uint256)"
        ])?
    );

    // Encode call to balanceOf function.
    let owner_str = String::from("0xe0e0Bb15Ad2dC19e5Eaa133968e498B4D9bF24Da"); 
    let owner_addr = ethers::types::H160::from_str(&owner_str).unwrap(); 
    let owner_token = ethers::abi::Token::Address(owner_addr); 
    let encoded = abi.encode("balanceOf", owner_token)?; 
    print!("encoded : {:#?}", encoded); 

    // initialize new EthersDB
    let mut ethersdb = EthersDB::new(Arc::clone(&client), None).unwrap();

    // query basic properties of an account incl bytecode
    let acc_info = ethersdb.basic(pool_address).unwrap().unwrap();

    // query value of storage slot at account address
    let value = ethersdb.storage(pool_address, slot_index).unwrap();

    // initialise empty in-memory-db
    let mut cache_db = CacheDB::new(EmptyDB::default());

    // insert basic account info which was generated via Web3DB with the corresponding address
    cache_db.insert_account_info(pool_address, acc_info);

    // insert our pre-loaded storage slot to the corresponding contract key (address) in the DB
    cache_db
        .insert_account_storage(pool_address, slot_index, value)
        .unwrap();

    // initialise an empty (default) EVM
    let mut evm = EVM::new();

    // insert pre-built database from above
    evm.database(cache_db);

    // fill in missing bits of env struct
    // change that to whatever caller you want to be
    evm.env.tx.caller = address!("0000000000000000000000000000000000000000");
    // account you want to transact with
    evm.env.tx.transact_to = TransactTo::Call(pool_address);
    // calldata formed via abigen
    evm.env.tx.data = encoded.0.into();
    // transaction value in wei
    evm.env.tx.value = U256::from(0);

    // execute transaction without writing to the DB
    let ref_tx = evm.transact_ref().unwrap();
    // select ExecutionResult struct
    let result = ref_tx.result;

    // unpack output call enum into raw bytes
    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => panic!("Execution failed: {result:?}"),
    };

    println!("value : {:#?}",value); 

    Ok(())
}