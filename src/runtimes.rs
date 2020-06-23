
use sp_runtime::{
    generic::Header,
    traits::{
        BlakeTwo256,
        IdentifyAccount,
        Verify,
    },
    MultiSignature,
    OpaqueExtrinsic,
};
use frame_support::Parameter;

use sub_runtime::ipse::Trait as IpseTrait;
use substrate_subxt::{
    balances::{
        AccountData,
        Balances,
    },
    system::System,
};
use sp_runtime::traits::{AtLeast32Bit, Scale};

pub trait Timestamp: System {
    type Moment: Parameter + Default + AtLeast32Bit
    + Scale<Self::BlockNumber, Output = Self::Moment> + Copy;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IpseRuntime;

impl System for IpseRuntime {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
    type Address = pallet_indices::address::Address<Self::AccountId, u32>;
    type Header = Header<Self::BlockNumber, BlakeTwo256>;
    type Extrinsic = OpaqueExtrinsic;
    type AccountData = AccountData<<Self as Balances>::Balance>;
}

impl Timestamp for IpseRuntime {
    type Moment = u128;
}

impl Balances for IpseRuntime {
    //type Balance = u128;
    type Balance = <Self as Balances>::Balance;
}

impl IpseTrait for IpseRuntime {
    type Event = <Self as System>::Event;
    type Currency = <Self as Balances>::Balance;
}
