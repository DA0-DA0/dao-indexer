use cosmrs::tx::Tx;
use diesel::associations::HasTable;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use tendermint_rpc::Client;
use tendermint_rpc::{HttpClient as TendermintClient};
use crate::db::models::{NewBlock, Block};
use crate::db::schema::block::dsl::*;


// going to need async, the main things we'll need are the link to the database and 
pub async fn blocker(db: &PgConnection) {
    // Instantiate tendermint client
    // begin reading database for completed chunks, and find the missing chunks
    let tendermint_client = TendermintClient::new("http://127.0.0.1:26657").unwrap();

    let latest_block_response = tendermint_client.latest_block_results().await.unwrap();
    let latest_block_height = latest_block_response.height.value();


    let zeta = block.load::<Block>(db);
    
    // //! let users = users::table.load::<User>(&connection)?;
    // block::table()
    println!("Latest block height: {}", latest_block_height);

    // Query the database for any heights missing if so, 
    // add to a sequence of blocks we will be querying from.

    
    // block.
    // block.load::<Block>(&db);

    // Height missing can be seen as getting the current height and querying all those blocks
    // for 1..latest_block_height {
        
        // posts.limit(1).load::<Post>
        //    let results = posts
        // .filter(published.eq(true))
        // .limit(5)
        // .load::<Post>(&connection)
        // .expect("Error loading posts");
    // }

    for block_height in 1..10 {
        let response = tendermint_client.block(block_height as u32).await.unwrap();
        println!("{}", response.block_id.hash);

        let new_block = NewBlock {
            height: response.block.header.height.value() as i64,
            hash: &response.block_id.hash.to_string(),
            num_txs: response.block.data.iter().len() as i64,
        };

        diesel::insert_into(block)
            .values(&new_block)
            .execute(&*db)
            .expect("Error saving new Block");


        for tx in response.block.data.iter() {
            let unmarshalled_tx = Tx::from_bytes(tx.as_bytes()).unwrap();
            for tx_message in unmarshalled_tx.body.messages {
                // TODO, attach here gavins code to index based on the type of transaction
                classify_transaction(tx_message)
            }
        }
    }
}


fn classify_transaction(tx: cosmrs::Any) {
    match tx.type_url.to_string().as_str() {
        "/cosmwasm.wasm.v1.MsgInstantiateContract" => {
            println!("we found an instnatiate contract, p0g")
        }
        _ => {
            println!("No handler for {}", tx.type_url.to_string().as_str());
        }
    }
}