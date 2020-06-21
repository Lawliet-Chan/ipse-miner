use crate::config::Conf;
use crate::storage::*;
use crate::error::IpseError;
use codec::Encode;
use frame_support::{StorageHasher, Twox64Concat};
use frame_system::ensure_signed;
use keccak_hasher::KeccakHasher;
use rusqlite::{params, Connection};

use frame_support::traits::Len;
use sp_core::{storage::StorageKey, twox_128, ed25519::{Pair, Public}};
use sp_runtime::{SaturatedConversion, AccountId32};
use sub_runtime::ipse::{BalanceOf, Order};
use substrate_subxt::{
    system::System, Call, Client, ClientBuilder, Error as SubError,
};
use sub_runtime::ipse::Miner as SubMiner;
use crate::runtimes::IpseRuntime as Runtime;
use triehash::ordered_trie_root;

type AccountId = <Runtime as System>::AccountId;
type Balance = BalanceOf<Runtime>;

const IPSE_MODULE: &str = "Ipse";
const ORDERS_STORAGE: &str = "Orders";
const MINERS_STORAGE: &str = "Miners";
const REGISTER_MINER: &str = "register_miner";
const CONFIRM_ORDER: &str = "confirm_order";
const DELETE: &str = "delete";

pub const SECTOR_SIZE: u64 = 128 * 1024 * 1024;

pub struct Miner<S: Storage> {
    nickname: &'static str,
    region: &'static str,
    url: &'static str,
    capacity: u64,
    unit_price: u64,
    signer: Pair,
    cli: Client<Runtime>,
    meta_db: Connection,
    storage: S,
}

#[derive(Debug)]
pub struct DataInfo {
    pub order: u64,
    pub sector: u64,
    pub length: u64,
    // In IPFS, it is hash.
    pub file_url: String,
}

#[derive(Debug)]
pub struct SectorInfo {
    pub sector: u64,
    // remaining storage capacity
    pub remain: u64,
}

impl<S: Storage> Miner<S> {
    pub fn new(cfg: Conf) -> Self {
        let meta_db = Connection::open(cfg.meta_path).expect("open sqlite failed");
        meta_db
            .execute(
                "CREATE TABLE IF NOT EXISTS data_info (\
            order  BIGINT PRIMARY KEY,\
            sector BIGINT NOT NULL,\
            length BIGINT NOT NULL,\
            file_url TEXT NOT NULL\
            )",
                params![],
            )
            .expect("init DataInfo table failed");
        meta_db
            .execute(
                "CREATE TABLE IF NOT EXISTS sector_info (\
            sector  BIGINT AUTO_INCREMENT,\
            remain  BIGINT DEFAULT ?\
            )",
                &[SECTOR_SIZE as i64],
            )
            .expect("init SectorInfo table failed");

        let storage = new_storage(cfg.ipfs_url);
        let cli = async_std::task::block_on(async move {
            ClientBuilder::<Runtime>::new()
                .set_url(cfg.chain_url)
                .build()
                .await
                .unwrap()
        });
        let signer =
            Pair::from_legacy_string(&format!("//{}", cfg.sign), cfg.pwd);

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

    pub fn write_file(&self, id: u64, file: Vec<u8>) -> Result<(), IpseError> {
        let f_len = file.len();

        let file_url = self.storage.write(file)?;

        let mut stmt = self
            .meta_db
            .prepare("SELECT sector FROM sector_info WHERE remain >= :size")?;
        let rows = stmt.query_map_named(&[(":size", &f_len)], |row| row.get(0))?;
        let sector_to_fill: u64 = if rows.len() == 0 {
            self.meta_db.execute(
                "INSERT INTO sector_info (remain) VALUES (?1)",
                &[SECTOR_SIZE],
            )?;
            let mut stmt = self.meta_db.prepare("SELECT COUNT(*) FROM sector_info")?;
            let count_rows = stmt.query_map_named(params![], |row| row.get(0))?;
            count_rows[0]?
        } else {
            rows[0]?
        };

        let data_info = DataInfo {
            order: id,
            sector: sector_to_fill,
            length: f_len as u64,
            file_url,
        };

        self.meta_db.execute(
            "INSERT INTO data_info (order, sector, length, file_url) VALUES (?1, ?2, ?3, ?4)",
            params![
                data_info.order,
                data_info.sector,
                data_info.length,
                data_info.file_url
            ],
        )?;
        self.meta_db.execute(
            "UPDATE sector_info SET remain = remain - ?1 WHERE sector = ?2",
            &[f_len, sector_to_fill],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: u64) -> Result<(), IpseError> {
        let mut stmt = self
            .meta_db
            .prepare("SELECT sector, length, file_url FROM data_info WHERE order = :order")?;
        let rows = stmt.query_map_named(&[(":order", &id)], |row| {
            Ok(DataInfo {
                order: id,
                sector: row.get(0)?,
                length: row.get(1)?,
                file_url: row.get(2)?,
            })
        })?;
        let data_info: DataInfo = rows[0]?;
        let file_url = "/ipfs/".to_string() + data_info.file_url.as_str();
        self.storage.delete(file_url.as_str())?;
        self.meta_db.execute(
            "UPDATE sector_info SET remain = remain + ?1 WHERE sector = ?2",
            &[data_info.length, data_info.sector],
        )?;
        Ok(())
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
        let account_id: AccountId32 =
            Self::to_account_id(signer);
        let mut storage_key = twox_128(IPSE_MODULE.as_ref()).to_vec();
        storage_key.extend(twox_128(MINERS_STORAGE.as_ref()).to_vec());
        storage_key.extend(
            account_id
                .encode()
                .using_encoded(Twox64Concat::hash),
        );
        let miner_key = StorageKey(storage_key);
        async_std::task::block_on(async move {
            let miner_opt: Option<Option<SubMiner<Balance>>> = self.cli.fetch(miner_key, None).await.unwrap();
            if let Some(mi) = miner_opt {
                mi.is_some()
            } else {
                false
            }

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
        let call = Call::new(IPSE_MODULE, DELETE, DeleteArgs { id: id as u64 });
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

    fn to_account_id(p: Pair) -> AccountId32 {
        let p = *Public::from(p).as_array_ref();
        p.into()
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
