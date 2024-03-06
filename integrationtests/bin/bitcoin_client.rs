use bitcoincore_rpc::{json::AddressType, Auth, Client, RpcApi};

pub struct BitcoinClient {}

pub fn get_block_height() -> u64 {
    let rpc = Client::new(
        "http://localhost:18443",
        Auth::UserPass("polaruser".to_string(), "polarpass".to_string()),
    )
    .unwrap();

    // let wallet = rpc
    //     .create_wallet("mywallet", None, None, None, None)
    //     .unwrap();

    // let new_address = rpc
    //     .get_new_address(None, Some(AddressType::Bech32m))
    //     .unwrap();
    // println!("best block hash: {:?}", new_address);

    // let out = rpc
    //     .generate_to_address(2, &new_address.assume_checked())
    //     .unwrap();
    let gen_result = rpc.generate(108, Some(200));

    //println!("out: {:?}", out);

    let w_info = rpc.get_wallet_info().unwrap();
    println!("wallet info: {:?}", w_info);
    let height = rpc.get_block_count();
    println!("height: {:?}", height);

    1
}

pub fn fund_lnd(){
    
}

fn main() {
    get_block_height();
}
