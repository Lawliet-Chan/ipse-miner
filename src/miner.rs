use crate::config::Conf;
use crate::storage::*;
use crate::error::IpseError;
use codec::Encode;
use frame_support::{StorageHasher, Twox64Concat};
use keccak_hasher::KeccakHasher;
use rusqlite::{params, Connection};

use sp_core::{ sr25519::{Pair, Public}};
use sp_keyring::AccountKeyring;
use sp_runtime::{SaturatedConversion, AccountId32};
use sub_runtime::ipse::{Order};
use substrate_subxt::{
    Client, ClientBuilder, Error as SubError, PairSigner,
};
use sub_runtime::ipse::Miner as SubMiner;
use triehash::ordered_trie_root;
use crate::storage::ipfs::IpfsStorage;
use crate::calls::{
    IpseRuntime as Runtime, AccountId, Balance,
    MinersStoreExt, OrdersStoreExt,
    RegisterMinerCallExt, ConfirmOrderCallExt, DeleteCallExt
};

pub const SECTOR_SIZE: u64 = 128 * 1024 * 1024;

pub struct Miner {
    nickname: &'static str,
    region: &'static str,
    url: &'static str,
    capacity: u64,
    unit_price: u64,
    pair_signer: PairSigner<Runtime, Pair>,
    cli: Client<Runtime>,
    meta_db: Connection,
    storage: IpfsStorage,
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

impl Miner {
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

        let storage  = new_ipfs_storage(cfg.ipfs_url.as_str());
        let cli = async_std::task::block_on(async move {
            ClientBuilder::<Runtime>::new()
                .set_url(cfg.chain_url.as_str())
                .build()
                .await
                .unwrap()
        });
        let pwd = if let Some(pwd) = cfg.pwd {
            Some(pwd.as_str())
        } else { None };
        let pair_signer = PairSigner::new(AccountKeyring::Alice.pair());
            //Pair::from_legacy_string(&format!("//{}", cfg.sign), pwd);

        let miner = Self {
            nickname: cfg.nickname.as_str(),
            region: cfg.region.as_str(),
            url: cfg.url.as_str(),
            capacity: cfg.capacity,
            unit_price: cfg.unit_price,
            pair_signer,
            cli,
            meta_db,
            storage,
        };

        miner.register_miner();
        miner
    }

    pub fn write_file(&self, id: u64, file: Vec<u8>) -> Result<(), IpseError> {
        let order_opt = self.get_order_from_chain(id as usize)?;
        if let Some(order) = order_opt {
            let merkle_root_on_chain = order.merkle_root;
            if self.check_merkle_root(&file, merkle_root_on_chain) {
                self.do_write_file(id, file)?;
            }
        }
        Ok(())
    }

    pub fn delete_file(&self, id: u64) -> Result<(), IpseError> {
        self.do_delete_file(id)
    }

    fn do_write_file(&self, id: u64, file: Vec<u8>) -> Result<(), IpseError> {
        let f_len = file.len();

        let file_url = self.storage.write(file)?;

        let mut stmt = self
            .meta_db
            .prepare("SELECT sector FROM sector_info WHERE remain >= :size")?;
        let rows = stmt.query_map_named(&[(":size", &f_len)], |row| row.get(0))?;
        let sector_to_fill: u64 = if rows.count() == 0 {
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

        self.meta_db.execute(
            "INSERT INTO data_info (order, sector, length, file_url) VALUES (?1, ?2, ?3, ?4)",
            params![
                id,
                sector_to_fill,
                f_len,
                file_url,
            ],
        )?;
        self.meta_db.execute(
            "UPDATE sector_info SET remain = remain - ?1 WHERE sector = ?2",
            &[f_len, sector_to_fill as usize],
        )?;
        Ok(())
    }

    fn do_delete_file(&self, id: u64) -> Result<(), IpseError> {
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
        //if !self.exist_miner_on_chain() {
        self.call_register_miner()
            .expect("register miner to chain failed")
        //}
    }

    fn check_merkle_root(&self, file: &Vec<u8>, merkle_root_on_chain: [u8; 32]) -> bool {
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
        async_std::task::block_on(async move {
            let orders: Vec<Order<AccountId, Balance>> = self.cli.orders(None).await?;
            Ok(orders.get(id))
        })
    }

    // pub fn exist_miner_on_chain(&self) -> bool {
    //     let signer = PairSigner::new(AccountKeyring::Alice.pair());
    //     let account_id: AccountId32 =
    //         Self::to_account_id(signer);
    //     async_std::task::block_on(async move {
    //
    //     })
    // }

    fn call_register_miner(&self) -> Result<(), SubError> {
        async_std::task::block_on(async move {
            let signer = PairSigner::new(AccountKeyring::Alice.pair());
            self.cli.register_miner(
                &signer,
                self.nickname.as_bytes().to_vec(),
                self.region.as_bytes().to_vec(),
                self.url.as_bytes().to_vec(),
                self.capacity,
                self.unit_price.saturated_into::<Balance>(),
            ).await?;
            Ok(())
        })
    }

    fn call_confirm_order(&self, id: usize, url: String) -> Result<(), SubError> {
        async_std::task::block_on(async move {
            let signer = PairSigner::new(AccountKeyring::Alice.pair());
            self.cli.confirm_order(
                &signer,
                id as u64,
                url.into_bytes()
            ).await?;
            Ok(())
        })
    }

    fn call_delete(&self, id: usize) -> Result<(), SubError> {
        async_std::task::block_on(async move {
            let signer = PairSigner::new(AccountKeyring::Alice.pair());
            self.cli.delete(&signer, id as u64).await?;
            Ok(())
        })
    }
}
