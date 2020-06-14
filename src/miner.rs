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

type AccountId = <Runtime as System>::AccountId;
type Balance = BalanceOf<Runtime>;

const IPSE_MODULE: &str = "Ipse";
const ORDERS_STORAGE: &str = "Orders";
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

#[derive(Encode)]
pub struct ConfirmArgs {
    pub id: u64,
    pub url: Vec<u8>,
}

#[derive(Encode)]
pub struct DeleteArgs {
    pub id: u64
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
        /// This signer should be Configurable.
        let signer = AccountKeyring::Alice.pair();
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

    pub fn confirm_order_to_chain(&self, id: usize) {

    }

    pub fn delete_on_chain(&self, id: usize) {

    }

    pub fn write(&mut self) {

    }

    pub fn read(&self) {

    }

    pub fn delete(&mut self) {

    }

    fn call_confirm_order(&self, id: usize, url: String) -> Result<(), SubError>{
        let call = Call::new(IPSE_MODULE, CONFIRM_ORDER, ConfirmArgs{
            id: id as u64,
            url: url.into_bytes(),
        });
        async_std::task::block_on(async move {
            let signer = self.signer.clone();
            let xt = self.cli.xt(signer, None).await?;
            xt.watch().submit(call).await?;
            Ok(())
        })
    }

    fn call_delete(&self, id: usize) -> Result<(), SubError>{
        let call = Call::new(IPSE_MODULE, DELETE, DeleteArgs{
            id: id as u64,
        });
        async_std::task::block_on(async move {
            let signer = self.signer.clone();
            let xt = self.cli.xt(signer, None).await?;
            xt.watch().submit(call).await?;
            Ok(())
        })
    }

}