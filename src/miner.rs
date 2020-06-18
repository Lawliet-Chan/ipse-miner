use crate::config::Conf;
use crate::storage::*;
use codec::{Decode, Encode};
use frame_support::{StorageHasher, Twox64Concat};
use frame_system::ensure_signed;
use keccak_hasher::KeccakHasher;
use rusqlite::{Connection, params};

use sp_core::{storage::StorageKey, twox_128, Pair};
use sp_keyring::AccountKeyring;
use sp_runtime::SaturatedConversion;
use sub_runtime::ipse::{BalanceOf, Order};
use substrate_subxt::{
    system::System, Call, Client, ClientBuilder, DefaultNodeRuntime as Runtime, Error as SubError,
    ExtrinsicSuccess,
};
use triehash::ordered_trie_root;
use std::usize::{from_be_bytes, to_be_bytes};

type AccountId = <Runtime as System>::AccountId;
type Balance = BalanceOf<Runtime>;

const IPSE_MODULE: &str = "Ipse";
const ORDERS_STORAGE: &str = "Orders";
const MINERS_STORAGE: &str = "Miners";
const REGISTER_MINER: &str = "register_miner";
const CONFIRM_ORDER: &str = "confirm_order";
const DELETE: &str = "delete";

pub const SECTOR_SIZE: u64 = 128 * 1024 * 1024;

pub struct Miner<S: Storage, P: Pair> {
    nickname: &'static str,
    region: &'static str,
    url: &'static str,
    capacity: u64,
    unit_price: u64,
    signer: P,
    cli: Client<Runtime>,
    meta_db: Connection,
    storage: S,
}

#[derive(Debug)]
pub struct DataInfo {
    pub order: u64,
    pub sector: u64,
    pub offset: u64,
    pub length: usize,
    // In IPFS, it is hash.
    pub file_url: String,
}

#[derive(Debug)]
pub struct SectorInfo {
    pub sector: u64,
    // remaining storage capacity
    pub remain: u64,
}

impl<S: Storage, P: Pair> Miner<S, P> {
    pub fn new(cfg: Conf) -> Self {
        let meta_db = Connection::open(cfg.meta_path).expect("open sqlite failed");
        meta_db.execute("CREATE TABLE data_info (\
            order  BIGINT PRIMARY KEY,\
            sector BIGINT,\
            offset BIGINT,\
            length BIGINT,\
            file_url TEXT NOT NULL\
            )", params![])
            .expect("init DataInfo table failed");
        meta_db.execute("CREATE TABLE sector_info (\
            sector  BIGINT PRIMARY KEY,\
            remain  BIGINT\
            )", params![])
            .expect("init SectorInfo table failed");
        let storage = new_storage::<ipfs::IpfsStorage>(cfg.ipfs_url);
        let cli = async_std::task::block_on(async move {
            ClientBuilder::<Runtime>::new()
                .set_url(cfg.chain_url)
                .build()
                .await
                .unwrap()
        });
        let signer =
            Pair::from_string(&format!("//{}", cfg.sign), cfg.pwd).expect("make Pair failed");
        let miner = Self {
            nickname: cfg.nickname,
            region: cfg.region,
            url: cfg.url,
            capacity: cfg.capacity,
            unit_price: cfg.unit_price,
            signer,
            cli,
            meta_db,
            storage,
        };

        miner.register_miner();
        miner
    }

    pub fn write_file(&self, id: usize, file: Vec<u8>) -> Result<(), IpseError>{

    }

    pub fn delete(&self, id: usize) -> Result<(), IpseError>{

    }

    fn register_miner(&self) {
        if !self.exist_miner_on_chain() {
            self.call_register_miner()
                .expect("register miner to chain failed")
        }
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

    fn get_order_from_chain(
        &self,
        id: usize,
    ) -> Result<Option<&Order<AccountId, Balance>>, SubError> {
        let mut storage_key = twox_128(IPSE_MODULE.as_ref()).to_vec();
        storage_key.extend(twox_128(ORDERS_STORAGE.as_ref()).to_vec());
        let order_key = StorageKey(storage_key);
        async_std::task::block_on(async move {
            let orders_opt: Option<Vec<Order<AccountId, Balance>>> =
                self.cli.fetch(order_key, None).await?;
            if let Some(orders) = orders_opt {
                Ok(orders.get(id))
            } else {
                Ok(None)
            }
        })
    }

    pub fn exist_miner_on_chain(&self) -> bool {
        let signer = self.signer.clone();
        let account_id: AccountId =
            ensure_signed(signer).expect("parse signer into accountID failed");
        let mut storage_key = twox_128(IPSE_MODULE.as_ref()).to_vec();
        storage_key.extend(twox_128(MINERS_STORAGE.as_ref()).to_vec());
        storage_key.extend(
            account_id
                .as_ref()
                .encode()
                .using_encoded(Twox64Concat::hash),
        );
        let miner_key = StorageKey(storage_key);
        async_std::task::block_on(async move {
            let miner_opt: Option<_> = self.cli.fetch(miner_key, None).await.unwrap();
            miner_opt.is_some()
        })
    }

    fn call_register_miner(&self) -> Result<(), SubError> {
        let call = Call::new(
            IPSE_MODULE,
            REGISTER_MINER,
            RegisterArgs {
                nickname: self.nickname.as_bytes().to_vec(),
                region: self.region.as_bytes().to_vec(),
                url: self.url.as_bytes().to_vec(),
                capacity: self.capacity,
                unit_price: self.unit_price.saturated_into::<Balance>(),
            },
        );
        self.async_call_chain(call)
    }

    fn call_confirm_order(&self, id: usize, url: String) -> Result<(), SubError> {
        let call = Call::new(
            IPSE_MODULE,
            CONFIRM_ORDER,
            ConfirmArgs {
                id: id as u64,
                url: url.into_bytes(),
            },
        );
        self.async_call_chain(call)
    }

    fn call_delete(&self, id: usize) -> Result<(), SubError> {
        let call = Call::new(IPSE_MODULE, DELETE, DeleteArgs{
            id: id as u64,
        });
        self.async_call_chain(call)
    }

    fn async_call_chain<C: Encode>(&self, call: Call<C>) -> Result<(), SubError> {
        async_std::task::block_on(async move {
            let signer = self.signer.clone();
            let xt = self.cli.xt(signer, None).await?;
            xt.watch().submit(call).await?;
            Ok(())
        })
    }
}

#[derive(Debug)]
pub enum IpseError {
    IO(std::io::Error),
    Sqlite(rusqlite::Error),
    IpfsResp(ipfs_api::response::Error),
}

impl From<std::io::Error> for IpseError {
    fn from(err: std::io::Error) -> Self {
        IpseError::IO(err)
    }
}

impl From<rusqlite::Error> for IpseError {
    fn from(err: rusqlite::Error) -> Self {
        IpseError::rusqlite(err)
    }
}

impl From<ipfs_api::response::Error> for IpseError {
    fn from(err: ipfs_api::response::Error) -> Self {
        IpseError::IpfsResp(err)
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
    pub id: u64,
}
