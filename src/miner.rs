use crate::storage::*;
use sled::Db;
use crate::config::Conf;
use substrate_subxt::{
    system::System,
    ExtrinsicSuccess,
    Call,
    Error as SubError,
    Client,
    DefaultNodeRuntime as Runtime,
    ClientBuilder,
};
use codec::{Encode};
use sp_core::{storage::StorageKey, twox_128, Pair};
use sp_keyring::AccountKeyring;
use sub_runtime::ipse::{Order, BalanceOf};
use triehash::ordered_trie_root;
use keccak_hasher::KeccakHasher;
use sp_runtime::SaturatedConversion;

type AccountId = <Runtime as System>::AccountId;
type Balance = BalanceOf<Runtime>;

const IPSE_MODULE: &str = "Ipse";
const ORDERS_STORAGE: &str = "Orders";
const REGISTER_MINER: &str = "register_miner";
const CONFIRM_ORDER: &str = "confirm_order";
const DELETE: &str = "delete";

pub struct Miner<S: Storage, P: Pair> {
    signer: P,
    cli: Client<Runtime>,
    meta_db: Db,
    storage: S,
}

struct DataInfo {
    pub sector: u64,
    pub offset: u64,
    pub length: u64,
}

impl<S: Storage, P: Pair> Miner<S, P>{
    pub fn new(cfg: Conf) -> Self {
        let meta_db = sled::open(cfg.meta_path).expect("open metadata db");
        let storage = new_storage::<ipfs::IpfsStorage>(cfg.ipfs_url);
        let cli = async_std::task::block_on(async move {
            ClientBuilder::<Runtime>::new()
                .set_url(url)
                .build().await.unwrap()
        });
        let signer = Pair::from_string(&format!("//{}", cfg.sign), cfg.pwd)
            .expect("make Pair failed");

        Self {
            signer,
            cli,
            meta_db,
            storage,
        }
    }

    pub fn get_order_from_chain(&self, id: usize) -> Option<&Order<AccountId, Balance>>{
        let mut storage_key = twox_128(IPSE_MODULE.as_ref()).to_vec();
        storage_key.extend(twox_128(ORDERS_STORAGE.as_ref()).to_vec());
        let order_key = StorageKey(storage_key);
        async_std::task::block_on(async move {
            let orders_opt: Option<Vec<Order<AccountId, Balance>>> = self.cli.fetch(order_key, None).await.unwrap();
            if let Some(orders) = orders_opt {
                orders.get(id)
            } else { None }
        })
    }

    pub fn write_file(&mut self, id: usize, file: Vec<u8>) {

    }

    pub fn read_file(&self, id: usize) {

    }

    pub fn delete_file(&mut self, id: usize) {
        self.call_delete(id);
    }

    fn check_merkle_root(&self, file: Vec<u8>, merkle_root_on_chain: [u8; 32]) -> bool {
        let mut iter = file.chunks(64);
        let mut chunks = Vec::new();
        while let Some(chunk) = iter.next() {
            chunks.push(chunk)
        }
        let merkle_root = ordered_trie_root::<KeccakHasher, _>(chunks);
        merkle_root == merkle_root_on_chain
    }

    fn call_register_miner(&self, nickname: &str, region: &str, url: &str, capacity: u64, unit_price: u64) -> Result<(), SubError> {
        let call = Call::new(IPSE_MODULE, REGISTER_MINER, RegisterArgs {
            nickname: nickname.as_bytes().to_vec(),
            region: region.as_bytes().to_vec(),
            url: url.as_bytes().to_vec(),
            capacity,
            unit_price: unit_price.saturated_into::<Balance>(),
        });
        self.async_call_chain(call)
    }

    fn call_confirm_order(&self, id: usize, url: String) -> Result<(), SubError> {
        let call = Call::new(IPSE_MODULE, CONFIRM_ORDER, ConfirmArgs{
            id: id as u64,
            url: url.into_bytes(),
        });
        self.async_call_chain(call)
    }

    fn call_delete(&self, id: usize) -> Result<(), SubError> {
        let call = Call::new(IPSE_MODULE, DELETE, DeleteArgs{
            id: id as u64,
        });
        self.async_call_chain(call)
    }

    fn async_call_chain<C: Encode>(&self,call: Call<C>) -> Result<(), SubError> {
        async_std::task::block_on(async move {
            let signer = self.signer.clone();
            let xt = self.cli.xt(signer, None).await?;
            xt.watch().submit(call).await?;
            Ok(())
        })
    }
}

#[derive(Encode)]
pub struct RegisterArgs {
    pub nickname: Vec<u8>,
    pub region: Vec<u8>,
    pub url: Vec<u8>,
    pub capacity: u64,
    pub unit_price: Balance,
}

#[derive(Encode)]
pub struct ConfirmArgs {
    pub id: u64,
    pub url: Vec<u8>,
}

#[derive(Encode)]
pub struct DeleteArgs {
    pub id: u64
}
