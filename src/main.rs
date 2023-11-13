

pub mod registry;
use ethers::abi::{ParamType, Token};
use ethers::providers::{Http, Provider};
use registry::{IExpressionDeployerV2, IParserV1, IInterpreterV1};
use std::sync::Arc;
use std::convert::TryFrom;
use std::str::FromStr;
// use ethers::contract::BaseContract; 
// use ethers::abi::parse_abi; 

use revm::primitives::Bytes;
use revm::primitives::keccak256;
use revm::{
    db::{CacheDB, EmptyDB, EthersDB},
    primitives::{address, ExecutionResult, Output, TransactTo, U256},
    Database, EVM,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // create ethers client and wrap it in Arc<M>
    let client = Provider::<Http>::try_from(
        "https://polygon.llamarpc.com",
    )?;
    let client = Arc::new(client); 

    let rain_interpreter_expression_deployer_v2 = ethers::types::H160::from_str(&String::from("0x808F049f53Ca70A2b8Ace4c117FFbec4ce77dE84")).unwrap();
    let expression_deployer = IExpressionDeployerV2::new(rain_interpreter_expression_deployer_v2.clone(), client.clone()); 

    let parser_contract = IParserV1::new(rain_interpreter_expression_deployer_v2.clone(), client.clone()); 

    let jittery_binomial =r#"
        input:123456,
        binomial18-10: decimal18-scale18<0>(bitwise-count-ones(bitwise-decode<0 10>(hash(input)))),
        noise18-1: int-mod(hash(input 0) 1e18),
        jittery-11: decimal18-add(binomial18-10 noise18-1),
        jittery-1: decimal18-div(jittery-11 11e18);
    "#.to_string(); 
    
    let (sources, constants) = parser_contract
        .parse(ethers::types::Bytes::from(jittery_binomial.as_bytes().to_vec()))
        .call()
        .await
        .unwrap(); 

    let min_outputs = vec![ethers::types::U256::from_dec_str("0").unwrap()]; 

    let deploy_expression = expression_deployer.deploy_expression(sources, constants, min_outputs) ; 
    let deploy_expression_bytes = deploy_expression.calldata().unwrap(); 

    let rain_interpreter_expression_deployer_v2 = address!("808F049f53Ca70A2b8Ace4c117FFbec4ce77dE84"); 
    let rain_interpreter_store = address!("97017811542bF3C4148847DC172d1a0F7e42342D"); 
    let ir = address!("BBE3275DCD2dF953362f6eC9dB1e44A9c9EF21F2");  


    // initialize new EthersDB
    let mut ethersdb = EthersDB::new(Arc::clone(&client), None).unwrap();

    // query basic properties of an account incl bytecode
    let acc_info = ethersdb.basic(rain_interpreter_expression_deployer_v2).unwrap().unwrap();
    let store_acc_info = ethersdb.basic(rain_interpreter_store).unwrap().unwrap(); 
    let ir_info = ethersdb.basic(ir).unwrap().unwrap(); 



    // println!("acc_info : {:#?}",acc_info);

    // initialise empty in-memory-db
    let mut cache_db = CacheDB::new(EmptyDB::default()); 

    // insert basic account info which was generated via Web3DB with the corresponding address
    cache_db.insert_account_info(rain_interpreter_expression_deployer_v2, acc_info);
    cache_db.insert_account_info(rain_interpreter_store, store_acc_info);
    cache_db.insert_account_info(ir, ir_info);
    


    // initialise an empty (default) EVM
    let mut evm = EVM::new();

    // insert pre-built database from above
    evm.database(cache_db);

    // fill in missing bits of env struct
    // change that to whatever caller you want to be
    evm.env.tx.caller = address!("53AB61eE41FA202227Eb4e7B176208FC626DC8A9");
    // account you want to transact with
    evm.env.tx.transact_to = TransactTo::Call(rain_interpreter_expression_deployer_v2);
    // calldata formed via abigen
    evm.env.tx.data = deploy_expression_bytes.0.into();
    // transaction value in wei
    evm.env.tx.value = U256::from(0);

    // execute transaction and write it to the DB
    let ref_tx = evm.transact_commit().unwrap();
    // select ExecutionResult struct
    let result = ref_tx;

    // unpack output call enum into raw bytes
    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => panic!("Execution failed: {result:?}"),
    };

    let decoded_data = ethers::abi::decode(&[ParamType::Address,ParamType::Address,ParamType::Address], &value).unwrap();

    let interpreter_deployer =  match &decoded_data[0] {
        Token::Address(address) => *address,
        _ => panic!("Expression not deployed")
    };  

    let store_deployer =  match &decoded_data[1] {
        Token::Address(address) => *address,
        _ => panic!("Expression not deployed")
    };  

    let expression_address =  match &decoded_data[2] {
        Token::Address(address) => *address,
        _ => panic!("Expression not deployed")
    };  
    
    let encode_dispatch = ethers::types::U256::from_dec_str("1902557528169978329301342351199330258480862274484126679039").unwrap();
    let statenamespace = ethers::types::U256::from_dec_str("0").unwrap();  

    let context: Vec<Vec<ethers::types::U256>> = vec![] ;   

    let interpreter = IInterpreterV1::new(interpreter_deployer.clone(), client.clone());  

    let tx = interpreter.eval(store_deployer, statenamespace, encode_dispatch, context);  
    let tx_bytes = tx.calldata().unwrap(); 

    println!("tx_bytes : {:#?}",tx_bytes); 

    let rain_interpreter_np = address!("BBE3275DCD2dF953362f6eC9dB1e44A9c9EF21F2"); 

   
    // fill in missing bits of env struct
    // change that to whatever caller you want to be
    evm.env.tx.caller = address!("53AB61eE41FA202227Eb4e7B176208FC626DC8A9");
    // account you want to transact with
    evm.env.tx.transact_to = TransactTo::Call(rain_interpreter_np);
    // calldata formed via abigen
    evm.env.tx.data = tx_bytes.0.into();
    // transaction value in wei
    evm.env.tx.value = U256::from(0);

    // execute transaction and write it to the DB
    let ref_tx = evm.transact_commit().unwrap();
    // select ExecutionResult struct
    let result = ref_tx;

    // unpack output call enum into raw bytes
    let value2 = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => panic!("Execution failed: {result:?}"),
    }; 

    let evaluable_config_tuple = ParamType::Tuple(
            [
                ParamType::Address,
                ParamType::Bytes,
                ParamType::Array(Box::new(ParamType::Uint(256))),
            ]
            .to_vec(),
        );

    let decoded_data = ethers::abi::decode(
        &[
            ParamType::Array(Box::new(ParamType::Uint(256))),
            ParamType::Array(Box::new(ParamType::Uint(256)))
        ], &value2).unwrap(); 

    println!("decoded_data : {:#?}",decoded_data);

    Ok(())
} 
